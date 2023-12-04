use std::{f32::consts::PI, str::FromStr};
use hifitime::prelude::*;
use sgp4::{Elements, Prediction};

use crate::tle::{self, get_elements_from_json};

const C: f32 = 299792458.0;


//takes in a point in rectangular coordinates, returns spherical coordinates
fn rect_to_spherical(r: &RectangularPoint) -> SphericalPoint{
    let rho = f32::sqrt(r.x.powi(2) + r.y.powi(2) + r.z.powi(2));
    let theta = f32::atan2(r.y, r.x);
    let phi = f32::atan2(f32::sqrt(r.x.powf(2.0) + r.y.powf(2.0)), r.z);
    return SphericalPoint { rho: rho, theta: theta, phi: phi}
}

fn spherical_to_rect(s: &SphericalPoint) -> RectangularPoint{
    let x = s.rho * f32::sin(s.phi) * f32::cos(s.theta);
    let y = s.rho * f32::sin(s.phi) * f32::sin(s.theta);
    let z = s.rho * f32::cos(s.phi);
    return RectangularPoint { x: x, y: y, z: z}
}

fn spherical_to_lat_lon(s: &SphericalPoint, time: Epoch) -> GroundPos {
    let lat = ((s.phi * 180.0 / PI) - 90.0) * -1.0;
    let sidereal_time = calc_gmst(time) as f32 / 86400.0 * 360.0;
    let mut lon = ((s.theta * 180.0 / PI) - sidereal_time) % 360.0;
    if lon < -180.0 {
        lon += 360.0;
    }
    if lon > 180.0 {
        lon -= 360.0;
    }
    
    return GroundPos { lat: lat, lon: lon}
}

//returns current gmst in seconds
pub fn calc_gmst(time: Epoch) -> f64 {
    let now = time;
    let s = (now.to_et_seconds() % 86400.0) - 43269.1839244;
    let t = (now.to_jde_et_days() - s / 86400.0 - 2451545.0) / 36525.0; //days since january 1, 4713 BC noon
    let h0 = 24110.54841 + 8640184.812866 * t + 0.093104 * t.powi(2); //the sidereal time at midnight this morning
    let h1 = 1.00273790935 + 5.9 * 10.0f64.powf(-11.0) * t;
    let rot = (h0 + h1 * s) % 86400.0;
    return rot;
}

#[tauri::command]
pub fn calc_gmst_now() -> f64 {
    return calc_gmst(Epoch::now().unwrap());
}



fn get_prediction(time: Epoch, elements: &Elements) -> Option<Prediction>{
    let epoch = Epoch::from_str(format!("{} UTC", elements.datetime).as_str()).unwrap();
        let duration = time - epoch;
        let constants = sgp4::Constants::from_elements(&elements).unwrap();
        //println!("last epoch was at {}", epoch);
        //println!("last epoch was {} ago", duration);
        let prediction = constants.propagate(duration.to_seconds() / 60 as f64);
        match prediction {
            Ok(pred) => return Some(pred),
            Err(_) => {
                //println!("{:?} at sat {}", e, elements.norad_id);
                return None
            }
        }

        //println!("        r = {:?} km", prediction.position);
        //println!("        ṙ = {:?} km.s⁻¹", prediction.velocity);
}

pub fn get_sat_lat_lon(time: Epoch, elements: &Elements) -> Option<GroundPos> {
    let pred_option = get_prediction(time, elements);
    if pred_option.is_some() {
        let pred = pred_option.unwrap();
        let x = pred.position.get(0).unwrap().clone() as f32;
        let y = pred.position.get(1).unwrap().clone() as f32;
        let z = pred.position.get(2).unwrap().clone() as f32;
        let rect = RectangularPoint{x: x, y: y, z: z};
        let spher = rect_to_spherical(&rect);
        let g = spherical_to_lat_lon(&spher, time);
        //println!("sat is at ({}, {}) at {:?}", g.lat, g.lon, time);
        return Some(g);
    } else {
        return None;
    }

}

#[tauri::command]
pub fn get_sat_lat(id: String) -> Result<f32, String>{
    match id.parse::<u32>() {
        Ok(idnum) => {
            let elements = get_elements_from_json(idnum).unwrap();
            return Ok(get_sat_lat_lon(Epoch::now().unwrap(), &elements).unwrap().lat)
        }
        Err(_) => return Err("failed to parse int".into())
    } 
}

#[tauri::command]
pub fn get_sat_lon(id: String) -> Result<f32, String>{
    match id.parse::<u32>() {
        Ok(idnum) => {
            let elements = get_elements_from_json(idnum).unwrap();
            return Ok(get_sat_lat_lon(Epoch::now().unwrap(), &elements).unwrap().lon)
        }
        Err(_) => return Err("failed to parse int".into())
    } 
}



fn get_user_position(earth_rad: i32, lat: f32, lon: f32, epoch: Epoch) -> RectangularPoint{
    let user_ra = (calc_gmst(epoch) as f32) / 86400.0 * 360.0 + lon;
    let user_position = SphericalPoint{rho: earth_rad as f32, phi: (90.0 - lat) * PI / 180.0, theta: user_ra * PI / 180.0};
    let user_rect = spherical_to_rect(&user_position);
    return user_rect
}

pub fn get_elavation(earth_rad: i32, lat: f32, lon: f32, elements: &Elements, epoch: Epoch) -> Option<f32>{
    let pred_option = get_prediction(epoch, elements);
    if pred_option.is_some() {
        let pred = pred_option.unwrap();
        let user_position = get_user_position(earth_rad, lat, lon, epoch);
        let user_to_sat_vec = RectangularPoint{x: pred.position.get(0).unwrap().clone() as f32 * 1000.0 - user_position.x, y: pred.position.get(1).unwrap().clone() as f32 * 1000.0 - user_position.y, z: pred.position.get(2).unwrap().clone() as f32 * 1000.0 - user_position.z};
        let user_mag = point_mag(&user_position);
        let user_to_sat_mag = f32::sqrt(user_to_sat_vec.x.powi(2) + user_to_sat_vec.y.powi(2) + user_to_sat_vec.z.powi(2));
        let user_dot_sat = user_position.x * user_to_sat_vec.x + user_position.y * user_to_sat_vec.y + user_position.z * user_to_sat_vec.z;
        let elevation = f32::acos(user_dot_sat / (user_mag * user_to_sat_mag));
        //println!("user ({}, {}, {}), user to sat ({}, {}, {})", user_position.x, user_position.y, user_position.z, user_to_sat_vec.x, user_to_sat_vec.y, user_to_sat_vec.z);
        return Some((PI/2.0 - elevation) * 180.0 / PI);
    } else {
        return None;
    }
    
}

fn get_azimuth(lat: f32, lon: f32, epoch: Epoch, elements: &Elements) -> Option<f32>{
    //to find the azimuth i will find the angle of intersection of two planes. The first plane will be the plane y=tan(theta)x, which is the plane that contains the user and the meridian they are in
    //the second plane will contain the user, the center of the earth, and the satellite

    let pred_option = get_prediction(epoch, elements);
    if pred_option.is_some() {
        let pred = pred_option.unwrap();
        let x = pred.position.get(0).unwrap().clone() as f32;
        let y = pred.position.get(1).unwrap().clone() as f32;
        let z = pred.position.get(2).unwrap().clone() as f32;
        let pred_rect = RectangularPoint {x:x, y:y, z:z};

        let up = RectangularPoint {x: 0.0, y: 0.0, z: 1.0};
        let user = get_user_position(6369555, lat, lon, epoch);
        let n1 = norm(&cross_product(&user, &up)); //normal vector of plane formed by earth's axis and the user
        let n2 = norm(&cross_product(&user, &pred_rect)); //normal vector of plane formed by center of earth, user, and sat

        let cos = n1.x * n2.x + n1.y * n2.y + n1.z * n2.z; //n1 dot n2
        let ang = f32::acos(cos) * 180.0 / PI;

        //find out whether the satellite is to the east or the west, since the angle between planes will always be between 0 and 180 deg.
        let g_option = get_sat_lat_lon(epoch, elements);
        if g_option.is_some() {
            let g = g_option.unwrap();
            let diff = (((g.lon - lon) % 360.0) + 360.0) % 360.0; //this looks it weird, it finds the modulus (instead of the remainder, which is what "%" actually finds)
            if diff < 180.0 {
                return Some(ang); //the satellite is east of the user, use the angle
            }
            else {
                return Some(360.0 - ang); //the satellite is to the west of the user, return the conjugate angle
            }
        } else {
            return None;
        }
 } else {
        return None;
    }

}

#[tauri::command]
pub fn get_alt(id: String) -> Result<f32, String>{
    match id.parse::<u32>() {
        Ok(idnum) => {
            let elements = get_elements_from_json(idnum).unwrap();
            let pred_option = get_prediction(Epoch::now().unwrap(), &elements);
            if pred_option.is_some() {
                let pred = pred_option.unwrap();
                let x = pred.position.get(0).unwrap().clone() as f32;
                let y = pred.position.get(1).unwrap().clone() as f32;
                let z = pred.position.get(2).unwrap().clone() as f32;
                let pred_rect = RectangularPoint {x:x, y:y, z:z};
                return Ok(point_mag(&pred_rect) - 6369.555);
            }
            else {
                return Err("no pred".into());
            }
        }
        Err(_) => return Err("failed to parse int".into())
    } 

}


fn calc_reciever_velocity(earth_rad: i32, lat: f32, lon: f32) -> RectangularPoint{
    let user = get_user_position(earth_rad, lat, lon, Epoch::now().unwrap());
    let r = f32::cos(lat * PI / 180.0) / earth_rad as f32; //find the radius of the circle that the user is revolving in (2d projection of path that they are travelling in);
    let user_dir = norm(&RectangularPoint { x: -user.y, y: user.x, z: 0.0 }); //unit vector in the direction of the velocity
    let user_speed = (2.0 * PI * r) / 86164.0; //circumference of the circle divided by rotational period of earth
    let user_vel = RectangularPoint {x: user_dir.x * user_speed, y: user_dir.y * user_speed, z: 0.0};
    return user_vel;
}

pub fn calc_doppler_shift(earth_rad: i32, lat: f32, lon: f32, frequency: f32, elements: &Elements) -> f32{
    // let wavelength: f32 = frequency / C;
    let pred_option = get_prediction(Epoch::now().unwrap(), elements);
    if pred_option.is_some() {
        let pred = pred_option.unwrap();
        let x = pred.position.get(0).unwrap().clone() as f32;
        let y = pred.position.get(1).unwrap().clone() as f32;
        let z = pred.position.get(2).unwrap().clone() as f32;
        let pred_rect = RectangularPoint {x:x, y:y, z:z};

        let vx = pred.velocity.get(0).unwrap().clone() as f32 * 1000.0;
        let vy = pred.velocity.get(1).unwrap().clone() as f32 * 1000.0;
        let vz = pred.velocity.get(2).unwrap().clone() as f32 * 1000.0;
        let pred_vel_rect = RectangularPoint {x:vx, y:vy, z:vz};

        let user = get_user_position(earth_rad, lat, lon, Epoch::now().unwrap());

        let sat_to_user = norm(&RectangularPoint{x: user.x - pred_rect.x , y: user.y - pred_rect.y, z: user.z - pred_rect.z});

        let user_vel = calc_reciever_velocity(earth_rad, lat, lon);
        let relative_vel = RectangularPoint{x: pred_vel_rect.x - user_vel.x, y: pred_vel_rect.y - user_vel.y, z: pred_vel_rect.z - user_vel.z};

    let cos = dot_product(&relative_vel, &sat_to_user) / (point_mag(&relative_vel) * point_mag(&sat_to_user));

        let new_freq = C / (C - point_mag(&relative_vel) * cos) * frequency;
        return new_freq - frequency;
    } else {
        return frequency;
    }
    
}

fn point_mag(rec: &RectangularPoint) -> f32 {
    return f32::sqrt(rec.x.powi(2) + rec.y.powi(2) + rec.z.powi(2));
}

fn cross_product(a: &RectangularPoint, b: &RectangularPoint) -> RectangularPoint{
    return RectangularPoint {x: a.y * b.z - a.z * b.y, y: a.z * b.x - a.x * b.z, z: a.x * b.y - a.y * b.x};
}

fn norm(rec: &RectangularPoint) -> RectangularPoint{
    let mag = point_mag(&rec);
    return RectangularPoint { x: rec.x / mag, y: rec.y / mag, z: rec.z / mag}
}

fn dot_product(a: &RectangularPoint, b: &RectangularPoint) -> f32{
    return a.x * b.x + a.y * b.y + a.z * b.z;
}

pub fn get_positions(minutes: i32, elements: &Elements) -> Vec<GroundPos> {
    let now = Epoch::now().unwrap();
    let mut positions: Vec<GroundPos> = vec![];
    for i in 0..(minutes*6) {
        let duration = Duration::from_seconds((i * 10) as f64);
        let epoch = now + duration;
        let g = get_sat_lat_lon(epoch, elements);
        positions.push(g.unwrap());
    }
    return positions;
}

fn lat_lon_to_x_y(g: &GroundPos) -> RectangularPoint {
    
    let width = 21600.0;
    let height = 10800.0;
    
    let x: f32 = ((g.lon + 180.0) * (width  / 360.0));
    let y: f32 = (((g.lat * -1.0) + 90.0) * (height / 180.0));
    return RectangularPoint { x: x, y: y, z: 0.0 };
}

#[tauri::command(async)]
pub fn get_all_sat_x_y() -> Result<Vec<Vec<i32>>, String> {
    //let elements_vec = tle::load_all_elements();
    let elements_vec = &tle::elements_static;
    let mut positions: Vec<Vec<i32>> = vec![];
    for elements in elements_vec.iter() {
                
        let ground_pos = get_sat_lat_lon(Epoch::now().unwrap(), elements);
        if ground_pos.is_some() {
                let r = lat_lon_to_x_y(&ground_pos.unwrap());
                positions.push(vec![r.x as i32, r.y as i32, elements.norad_id as i32]);

        }
    }
    return Ok(positions);
}

#[tauri::command(async)]
pub fn get_all_r() -> Result<Vec<Vec<i32>>, String> {
    //let elements_vec = tle::load_all_elements();
    let elements_vec = &tle::elements_static;
    let mut positions: Vec<Vec<i32>> = vec![];
    for elements in elements_vec.iter() {
                
        let pred_option = get_prediction(Epoch::now().unwrap(), elements);
        if pred_option.is_some() {
            let pred = pred_option.unwrap();
            let x = pred.position.get(0).unwrap().clone() as i32;
            let y = pred.position.get(1).unwrap().clone() as i32;
            let z = pred.position.get(2).unwrap().clone() as i32;
            positions.push(vec![x, y, z, elements.norad_id as i32]);
        }
    
    }
    println!("{:?}", positions[1]);
    return Ok(positions);
}

#[tauri::command]
pub fn get_name(id: String) -> Result<String, String>{
    match id.parse::<u32>() {
        Ok(idnum) => {
            let elements = get_elements_from_json(idnum).unwrap();
            return Ok(elements.object_name.unwrap())
        }
        Err(_) => return Err("failed to parse int".into())
    } 
}

#[tauri::command]
pub fn get_launch_date(id: String) -> Result<String, String> {
    match id.parse::<u32>() {
        Ok(idnum) => {
            let elements = get_elements_from_json(idnum).unwrap();
            let object_id = elements.international_designator.unwrap();
            let launch_year = object_id[0..4].into();
            return Ok(launch_year);
        }
        Err(_) => return Err("failed to parse int".into())
    }
}


struct SphericalPoint {
    rho: f32,
    theta: f32,
    phi: f32
}

pub struct RectangularPoint {
    pub x: f32,
    pub y: f32,
    pub z: f32
}

pub struct GroundPos {
    pub lat: f32,
    pub lon: f32
}

