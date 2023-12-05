const { invoke } = window.__TAURI__.tauri
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
const { WebviewWindow } = window.__TAURI__.window;
const { emit, listen } = window.__TAURI__.event;

let webview;

const scene = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(75, (window.innerWidth - 120) / (window.innerHeight - 30), 0.1, 10000000000);
const loader = new THREE.TextureLoader;
let satCoords = [];
let sats = [];
let selectedSatPath = [];
let selectedSatPathThree = [];
let satsCreated = false;
let earth;
let selected;
let selectedPrev;
let findInput;
const defaultMaterial = new THREE.MeshBasicMaterial({ color: 0x00FF00 });
const selectedMaterial = new THREE.MeshBasicMaterial({ color: 0xFF0000 });
const selectedGeo = new THREE.BoxGeometry(80, 80, 80);
const defaultGeo = new THREE.BoxGeometry(40, 40, 40);
const orbitPathMaterial = new THREE.LineBasicMaterial({ color: 0x0000FF });
let orbitPathGeometry = null;
let orbitPath = null;

const centerGeometry = new THREE.BoxGeometry(50, 50, 50);
const centerMaterial = new THREE.MeshBasicMaterial({ color: 0xff0000 });
const centerObject = new THREE.Mesh(centerGeometry, centerMaterial);


setInterval(updateSats, 100);

const renderer = new THREE.WebGLRenderer({ logarithmicDepthBuffer: true });
renderer.setSize(window.innerWidth - 120, window.innerHeight - 30);
let parent = document.body.querySelector(".tabcontent");
parent.appendChild(renderer.domElement);
renderer.domElement.id = 'globeCanvas';

const raycaster = new THREE.Raycaster();
const pointer = new THREE.Vector2();

const earthTexture = loader.load('/assets/earth.jpg');
const normalTexture = loader.load('/assets/Earth-normal-8k.jpg', makeEarth, console.log("progess"), error => console.log(error));
function makeEarth() {

    const earthmat = new THREE.MeshBasicMaterial({
        map: earthTexture,
        normalMap: normalTexture,

    });
    earth = new THREE.Mesh(
        new THREE.SphereGeometry(6369, 32, 32),
        earthmat
    );
    scene.add(earth);
    console.log("done");
}

function createSats() {
    const geometry = new THREE.BoxGeometry(40, 40, 40);
    const material = new THREE.MeshBasicMaterial({ color: 0x00ff00 });



    for (let i = 0; i < satCoords.length; i++) {
        const sat = new THREE.Mesh(geometry, material);
        sat.userData.id = satCoords[i][3];
        sats.push(sat);
        scene.add(sat);
    }
    scene.add(centerObject);

    //scene.add(new THREE.Line(orbitPathGeometry, orbitPathMaterial));

}

const controls = new OrbitControls(camera, renderer.domElement);
controls.rotateSpeed = 0.2;

camera.position.z = 20000;

function animate() {
    requestAnimationFrame(animate);

    centerObject.position.copy(controls.target);

    for (let i = 0; i < satCoords.length; i++) {
        sats[i].position.x = satCoords[i][1];
        sats[i].position.y = satCoords[i][2]; //for sure positive
        sats[i].position.z = satCoords[i][0];

        updateObjectScale(sats[i], camera);

        if (satCoords[i][3] == selected) {
            sats[i].material = selectedMaterial;
            sats[i].geometry = selectedGeo;
            if (selected != selectedPrev) {
                 updateSelectedPath(sats[i]);
            }
            selectedPrev = selected;
        } else {
            sats[i].material = defaultMaterial;
            sats[i].geometry = defaultGeo;
        }
    }
    invoke("calc_gmst_now").then((message) => {
        earth.rotation.y = (message / 86400.0 * 2 * Math.PI) - Math.PI / 2;
    })
    
    renderer.render(scene, camera);
}
animate();

function updateObjectScale(object, camera) {
    const distance = object.position.distanceTo(camera.position);
    const scaleFactor = distance / 10000; // Adjust this scale factor as needed

    if (distance > 10000) {
        object.scale.set(scaleFactor, scaleFactor, scaleFactor);
    } else {
        object.scale.set(1, 1, 1);
    }
}

async function updateSats() {
    invoke("get_all_r").then((message) => {
        satCoords = message;
        if (satsCreated == false) {
            createSats();
            satsCreated = true;
        }
    })

}

async function updateSelectedPath(satellite) {
    console.log("updatingSelectedPath");

    // Clear the old path
    if (orbitPath != null) {
        orbitPath.removeFromParent();
        orbitPathGeometry.dispose(); // Dispose of the old geometry
    }
    orbitPathGeometry = null;
    orbitPath = null;
    selectedSatPathThree = []; // Clear the array

    // Fetch and update the path data
    await getSatPath("" + satellite.userData.id);
    for (let i = 0; i < selectedSatPath.length; i++) {
        let x = selectedSatPath[i][1];
        let y = selectedSatPath[i][2];
        let z = selectedSatPath[i][0];
        selectedSatPathThree.push(new THREE.Vector3(x, y, z));
    }

    // Create and add the new path
    orbitPathGeometry = new THREE.BufferGeometry().setFromPoints(selectedSatPathThree);
    orbitPath = new THREE.Line(orbitPathGeometry, orbitPathMaterial)
    scene.add(orbitPath);
    console.log("Path updated for satellite ID:", satellite.userData.id);
}

async function getSatPath(id) {
    selectedSatPath = await invoke("get_orbit_path", { id: String(id) });
}

function updateFocusedObject() {
    // Calculate the focal point of the camera
    const distance = 10; // Distance in front of the camera
    const cameraDirection = new THREE.Vector3();
    camera.getWorldDirection(cameraDirection);
    const focalPoint = new THREE.Vector3().addVectors(camera.position, cameraDirection.multiplyScalar(distance));

    // Update the position of the object to the focal point
    focusedObject.position.copy(focalPoint);
}

window.addEventListener('resize', onWindowResize, false);

function onWindowResize() {

    camera.aspect = (window.innerWidth - 120) / (window.innerHeight - 30);
    camera.updateProjectionMatrix();

    renderer.setSize(window.innerWidth - 120, window.innerHeight - 30);

}

function select() {
    webview = new WebviewWindow('popup', {
        "width": 560,
        "height": 220,
        "url": "popup.html",
        "label": "popup",
        "title": "Info for selected satellite",
        "resizable": false,
        "alwaysOnTop": true
        
        
    });
    console.log("emitting selected", selected);
    
}

document.addEventListener("DOMContentLoaded", () => {
    const form = document.querySelector("#threed-form");
    if (form) {
        form.addEventListener("submit", (e) => {
            e.preventDefault(); // Prevent the default form submission
            const id = document.querySelector("#threed-input").value;
            const satellite = getSatelliteByID(id);
            if (satellite) {
                selected = id;
                findInput = selected;
                focusOnSatellite(satellite);
                select();
            } else {
                console.log("No satellite found with ID:", id);
            }
        });
    } else {
        console.error("Form not found");
    }
});

function getSatelliteByID(id) {
    for (let i = 0; i < sats.length; i++) {
        if (sats[i].userData.id == id) {
            return sats[i]; // Return the satellite mesh itself
        }
    }
    console.log("Satellite with ID", id, "not found");
    return null;
}

function focusOnSatellite(satellite) {
    if (satellite) {
        controls.target.copy(satellite.position);
        controls.update();
    } else {
        console.error("Invalid satellite object passed to focusOnSatellite");
    }
}

async function handleNeedSelected() {
    await listen('needselected', (event) => {
        emit('selected', {
            id: selected,
        })
    });
}

handleNeedSelected();



window.addEventListener('click', async (event) => {
    if (webview?.close) {
        webview.close();
    }
     const bounds = renderer.domElement.getBoundingClientRect();
    pointer.x = (((event.clientX - bounds.left) / bounds.width) * 2 - 1);
    pointer.y = (- ((event.clientY - bounds.top) / bounds.height) * 2 + 1);

    raycaster.setFromCamera(pointer, camera);
    const intersects = raycaster.intersectObjects(scene.children);
        
    for (let i = 0; i < intersects.length; i++) {
        if (intersects[i].object !== earth) {
            selected = intersects[i].object.userData.id;
            findInput = selected;
            focusOnSatellite(intersects[i].object);
            select();
            await updateSelectedPath(intersects[i].object); // Wait for the path to update
            break;
        } else {
            selected = null;
            findInput = null;
        }
    }

    if (intersects.length === 0 || intersects[0] == earth) {
        selected = null;
        findInput = null;
    }

})
