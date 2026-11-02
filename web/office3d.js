// Rizal Agent HQ — kantor virtual isometrik (Three.js, kamera ortografik).
// Gaya: terang & bersih — lantai kantor berperabot dengan dinding kaca,
// zona warna per tim, lingkungan luar (jalan, zebra cross, mobil berjalan,
// pohon, bangku), karakter low-poly lucu + chip nama melayang (HTML).
// API: initOffice(el, {onSelect, onHover}) -> { update(agents), preset(nama, agents), focusAgent(id) }
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';

const STATUS_COLORS = {
  'bekerja': 0x22b573, 'berpikir': 0x4a7dff, 'menunggu-rizal': 0xf07f13,
  'santai': 0x9aa7ba, 'offline': 0x6b7686, 'error': 0xe5484d,
};
const STATUS_CSS = {
  'bekerja': '#22b573', 'berpikir': '#4a7dff', 'menunggu-rizal': '#f07f13',
  'santai': '#9aa7ba', 'offline': '#6b7686', 'error': '#e5484d',
};
const STATUS_LABEL = {
  'bekerja': 'Bekerja', 'berpikir': 'Berpikir', 'menunggu-rizal': 'Menunggu Rizal',
  'santai': 'Santai', 'offline': 'Offline', 'error': 'Error',
};
const HAIR = [0x3a2e28, 0x6b4a2f, 0x22242a, 0x8a5a33, 0x4a4a52, 0x7a3b2e];

const mat = (c, o = {}) => new THREE.MeshStandardMaterial({ color: c, roughness: 0.85, metalness: 0.02, ...o });
function box(w, h, d, c, o) { const m = new THREE.Mesh(new THREE.BoxGeometry(w, h, d), mat(c, o)); m.castShadow = true; m.receiveShadow = true; return m; }
function cyl(rt, rb, h, c, seg = 14) { const m = new THREE.Mesh(new THREE.CylinderGeometry(rt, rb, h, seg), mat(c)); m.castShadow = true; return m; }
function sph(r, c, w = 14, h = 11) { const m = new THREE.Mesh(new THREE.SphereGeometry(r, w, h), mat(c)); m.castShadow = true; return m; }
function at(obj, x, y, z, ry = 0) { obj.position.set(x, y, z); obj.rotation.y = ry; return obj; }

function textBoard(text, w = 2.8, h = 0.85, bg = '#22304a', fg = '#ffffff') {
  const cv = document.createElement('canvas'); cv.width = 512; cv.height = Math.round(512 * h / w);
  const g = cv.getContext('2d');
  g.fillStyle = bg; g.fillRect(0, 0, cv.width, cv.height);
  g.fillStyle = fg; g.font = `700 ${Math.round(cv.height * 0.44)}px system-ui, sans-serif`;
  g.textAlign = 'center'; g.textBaseline = 'middle';
  g.fillText(text, cv.width / 2, cv.height / 2 + 2);
  const tex = new THREE.CanvasTexture(cv); tex.anisotropy = 4;
  return new THREE.Mesh(new THREE.PlaneGeometry(w, h), new THREE.MeshBasicMaterial({ map: tex }));
}
function chartScreen(w = 3.4, h = 1.9) {
  const cv = document.createElement('canvas'); cv.width = 512; cv.height = 288;
  const g = cv.getContext('2d');
  g.fillStyle = '#1d2739'; g.fillRect(0, 0, 512, 288);
  g.strokeStyle = '#33415e'; g.lineWidth = 1;
  for (let y = 40; y < 270; y += 38) { g.beginPath(); g.moveTo(30, y); g.lineTo(490, y); g.stroke(); }
  const cols = ['#3ec6b8', '#4a7dff', '#f2a541', '#e5484d', '#5fd68b', '#b49af2'];
  [90, 150, 70, 190, 120, 230].forEach((bh, i) => { g.fillStyle = cols[i]; g.fillRect(52 + i * 72, 268 - bh, 40, bh); });
  g.strokeStyle = '#ffd166'; g.lineWidth = 4; g.beginPath();
  [200, 160, 185, 120, 140, 80].forEach((y, i) => { const x = 72 + i * 72; i ? g.lineTo(x, y) : g.moveTo(x, y); });
  g.stroke();
  const tex = new THREE.CanvasTexture(cv); tex.anisotropy = 4;
  return new THREE.Mesh(new THREE.PlaneGeometry(w, h), new THREE.MeshBasicMaterial({ map: tex }));
}

// ---------- perabot ----------
function plant(scale = 1) {
  const g = new THREE.Group();
  g.add(at(cyl(0.16, 0.12, 0.3, 0xc96f4a), 0, 0.15, 0));
  g.add(at(sph(0.3, 0x4da65a), 0, 0.62, 0));
  g.add(at(sph(0.22, 0x5cb868), 0.18, 0.48, 0.1));
  g.add(at(sph(0.2, 0x43924f), -0.17, 0.5, -0.08));
  g.scale.setScalar(scale);
  return g;
}
function chair(color = 0x4a7dff) {
  const g = new THREE.Group();
  g.add(at(box(0.56, 0.09, 0.52, color), 0, 0.48, 0));
  g.add(at(box(0.56, 0.62, 0.09, color), 0, 0.82, -0.24));
  g.add(at(cyl(0.05, 0.05, 0.44, 0x8fa0b5), 0, 0.22, 0));
  g.add(at(cyl(0.26, 0.28, 0.05, 0xaebacb), 0, 0.03, 0));
  return g;
}
function desk(w = 2.0, color = 0x4a7dff, screen = 0xa8d8ff) {
  const g = new THREE.Group();
  g.add(at(box(w, 0.08, 0.95, 0xffffff), 0, 0.74, 0));
  g.add(at(box(0.08, 0.72, 0.85, 0xd7dee7), -w / 2 + 0.1, 0.37, 0));
  g.add(at(box(0.08, 0.72, 0.85, 0xd7dee7), w / 2 - 0.1, 0.37, 0));
  const mon = new THREE.Group();
  mon.add(at(box(0.98, 0.58, 0.05, 0x26303f), 0, 0.36, 0));
  const scr = new THREE.Mesh(new THREE.PlaneGeometry(0.9, 0.5), new THREE.MeshBasicMaterial({ color: screen }));
  scr.position.set(0, 0.36, 0.031); mon.add(scr);
  mon.add(at(box(0.08, 0.2, 0.08, 0x9aa7ba), 0, 0.08, 0));
  at(mon, 0, 0.78, -0.22); g.add(mon);
  g.add(at(box(0.72, 0.03, 0.26, 0xe6ecf3), 0, 0.8, 0.18));
  g.add(at(chair(color), 0, 0, 0.85));
  return g;
}
function tree(s = 1) {
  const g = new THREE.Group();
  g.add(at(cyl(0.09, 0.13, 0.9, 0x8a6b4f), 0, 0.45, 0));
  g.add(at(sph(0.62, 0x57ab5e, 12, 10), 0, 1.35, 0));
  g.add(at(sph(0.45, 0x67bb6d, 12, 10), 0.4, 1.05, 0.2));
  g.add(at(sph(0.4, 0x4c9a54, 12, 10), -0.38, 1.1, -0.15));
  g.scale.setScalar(s);
  return g;
}
function car(color) {
  const g = new THREE.Group();
  g.add(at(box(2.0, 0.42, 0.92, color), 0, 0.42, 0));
  g.add(at(box(1.05, 0.4, 0.84, color), -0.08, 0.8, 0));
  g.add(at(box(0.9, 0.3, 0.86, 0x2c3644), -0.08, 0.82, 0));
  const wheels = [];
  [[-0.68, 0.42], [0.68, 0.42], [-0.68, -0.42], [0.68, -0.42]].forEach(([x, z]) => {
    const w = cyl(0.17, 0.17, 0.1, 0x232a35); w.rotation.x = Math.PI / 2; at(w, x, 0.17, z); g.add(w); wheels.push(w);
  });
  g.userData.wheels = wheels;
  return g;
}
function bench() {
  const g = new THREE.Group();
  g.add(at(box(1.7, 0.07, 0.42, 0xb08968), 0, 0.45, 0));
  g.add(at(box(1.7, 0.3, 0.06, 0xb08968), 0, 0.72, -0.2));
  g.add(at(box(0.08, 0.45, 0.4, 0x5a6678), -0.7, 0.22, 0));
  g.add(at(box(0.08, 0.45, 0.4, 0x5a6678), 0.7, 0.22, 0));
  return g;
}

// ---------- karakter ----------
function makeHuman(color, seed) {
  const g = new THREE.Group();
  const dark = new THREE.Color(color).multiplyScalar(0.72).getHex();
  g.add(at(box(0.17, 0.52, 0.17, 0x3a465e), -0.12, 0.26, 0));
  g.add(at(box(0.17, 0.52, 0.17, 0x3a465e), 0.12, 0.26, 0));
  g.add(at(box(0.54, 0.64, 0.32, color), 0, 0.84, 0));
  const armGeo = new THREE.BoxGeometry(0.14, 0.56, 0.14); armGeo.translate(0, -0.2, 0);
  const armMat = mat(dark);
  const armL = new THREE.Mesh(armGeo, armMat); armL.castShadow = true; armL.position.set(-0.35, 1.04, 0.05);
  const armR = new THREE.Mesh(armGeo, armMat); armR.castShadow = true; armR.position.set(0.35, 1.04, 0.05);
  g.add(armL, armR);
  const head = new THREE.Group();
  head.add(at(sph(0.215, 0xf0bd93, 16, 13), 0, 0, 0));
  const hair = at(sph(0.22, HAIR[seed % HAIR.length], 16, 13), 0, 0.045, -0.015);
  hair.scale.set(1, 0.78, 1); head.add(hair);
  head.position.y = 1.38; g.add(head);
  g.userData.parts = { armL, armR, head };
  return g;
}
function makeRobot(color) {
  const g = new THREE.Group();
  g.add(at(cyl(0.3, 0.36, 0.1, 0x8fa0b5), 0, 0.1, 0));
  g.add(at(box(0.52, 0.58, 0.36, color), 0, 0.72, 0));
  g.add(at(box(0.4, 0.14, 0.05, 0xffffff), 0, 0.78, 0.19));
  const head = new THREE.Group();
  head.add(at(box(0.46, 0.4, 0.4, 0xf2f5f9), 0, 0, 0));
  const eyeMat = new THREE.MeshBasicMaterial({ color: 0x22304a });
  [-0.1, 0.1].forEach(x => { const e = new THREE.Mesh(new THREE.SphereGeometry(0.045, 8, 8), eyeMat); e.position.set(x, 0.03, 0.21); head.add(e); });
  head.add(at(cyl(0.02, 0.02, 0.26, 0x8fa0b5), 0, 0.32, 0));
  head.add(at(sph(0.055, color), 0, 0.48, 0));
  head.position.y = 1.24; g.add(head);
  g.add(at(box(0.11, 0.44, 0.11, color), -0.34, 0.72, 0));
  g.add(at(box(0.11, 0.44, 0.11, color), 0.34, 0.72, 0));
  g.userData.parts = { head };
  return g;
}

// slot berdiri per kategori agent: [x, z, rotY]
const SLOTS = {
  omp: [[3.5, 2.5, Math.PI]],
  Muse: [[-12.5, -6.4, Math.PI], [-9.6, -9.6, 0]],
  cursor: [[10.5, -6.4, Math.PI], [13.8, -9.6, 0]],
  mcp: [[15.3, 3.4, -Math.PI / 2]],
  extra: [[-3.4, -7.4, Math.PI / 2], [0.4, -9.8, 0], [-1.5, -6.9, -Math.PI / 2]],
  workers: [],
  agg: [[10.9, 10.3, Math.PI]],
  overflow: [],
};
[-2.5, 0.8, 4.1, 7.4].forEach(x => SLOTS.workers.push([x, 6.4, Math.PI]));
[-2.5, 0.8, 4.1, 7.4].forEach(x => SLOTS.workers.push([x, 10.0, Math.PI]));
for (let i = 0; i < 6; i++) SLOTS.overflow.push([-4 + i * 2.3, 11.6, Math.PI]);

const ZONES = {
  semua: { t: [0, 0], zoom: 0.74 },
  Muse: { t: [-12.5, -8], zoom: 1.3 },
  utama: { t: [3.5, 3.2], zoom: 1.25 },
  workers: { t: [2.5, 7.6], zoom: 1.3 },
  cursor: { t: [11, -8], zoom: 1.3 },
  meeting: { t: [-1.5, -8.5], zoom: 1.4 },
  mcp: { t: [15.5, 5.5], zoom: 1.45 },
};

export function initOffice(stageEl, { onSelect, onHover, onInteract, onMeetingChange } = {}) {
  const labelsEl = document.getElementById('labels');
  const renderer = new THREE.WebGLRenderer({ antialias: true });
  renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFSoftShadowMap;
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.12;
  stageEl.appendChild(renderer.domElement);

  const scene = new THREE.Scene();
  scene.background = new THREE.Color(0xe9edf2);
  const hemi = new THREE.HemisphereLight(0xffffff, 0xc9d4e2, 0.88);
  scene.add(hemi);
  const sun = new THREE.DirectionalLight(0xffffff, 2.15);
  sun.position.set(20, 32, 14);
  sun.castShadow = true;
  sun.shadow.mapSize.set(2048, 2048);
  Object.assign(sun.shadow.camera, { left: -34, right: 34, top: 30, bottom: -30, far: 90 });
  sun.shadow.bias = -0.0004;
  scene.add(sun);

  const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.1, 300);
  const ISO_DIR = new THREE.Vector3(1, 1, 1).normalize();
  let camDir = ISO_DIR.clone();
  camera.position.copy(ISO_DIR).multiplyScalar(70);
  camera.zoom = 0.74;

  const controls = new OrbitControls(camera, renderer.domElement);
  controls.enableDamping = true;
  controls.dampingFactor = 0.08;
  controls.maxPolarAngle = Math.PI / 2.15;
  controls.minZoom = 0.45; controls.maxZoom = 3.2;
  controls.target.set(0, 0, 0);

  // ===== lingkungan luar =====
  const world = new THREE.Group(); scene.add(world);
  const ground = new THREE.Mesh(new THREE.PlaneGeometry(170, 170), mat(0xdde4eb, { roughness: 1 }));
  ground.rotation.x = -Math.PI / 2; ground.position.y = -0.32; ground.receiveShadow = true; world.add(ground);
  const roadMat = mat(0x8e9aa8, { roughness: 1 });
  const roadS = new THREE.Mesh(new THREE.PlaneGeometry(170, 7.5), roadMat);
  roadS.rotation.x = -Math.PI / 2; at(roadS, 0, -0.3, 20.5); roadS.receiveShadow = true; world.add(roadS);
  const roadE = new THREE.Mesh(new THREE.PlaneGeometry(7.5, 170), roadMat);
  roadE.rotation.x = -Math.PI / 2; at(roadE, 26.5, -0.3, 0); roadE.receiveShadow = true; world.add(roadE);
  for (let x = -80; x < 82; x += 5) world.add(at(box(2.2, 0.02, 0.22, 0xf5f7fa, { roughness: 1 }), x, -0.27, 20.5));
  for (let z = -80; z < 82; z += 5) world.add(at(box(0.22, 0.02, 2.2, 0xf5f7fa, { roughness: 1 }), 26.5, -0.27, z));
  const walk = new THREE.Mesh(new THREE.PlaneGeometry(170, 2.6), mat(0xc7d0da, { roughness: 1 }));
  walk.rotation.x = -Math.PI / 2; at(walk, 0, -0.28, 15.6); walk.receiveShadow = true; world.add(walk);
  for (let i = 0; i < 5; i++) world.add(at(box(0.8, 0.02, 6.4, 0xf5f7fa, { roughness: 1 }), -15.4 + i * 1.5, -0.26, 20.5));
  [[-23, 13.5, 1.1], [23.5, 13.8, 1.25], [-23, -8, 1], [23.8, -9, 1.15], [-8, 14.6, 0.9], [14, 14.6, 1], [-23, 3, 1.2], [6, -15.5, 1.05], [-16, -15.5, 0.95]]
    .forEach(([x, z, s]) => world.add(at(tree(s), x, -0.3, z)));
  world.add(at(bench(), -19, -0.3, 14.8, 0.2), at(bench(), 19.5, -0.3, 14.8, -0.15));
  [[-20, 15.6], [0, 15.6], [20, 15.6]].forEach(([x, z]) => {
    const lamp = new THREE.Group();
    lamp.add(at(cyl(0.05, 0.07, 2.6, 0x5a6678), 0, 1.3, 0));
    const bulb = new THREE.Mesh(new THREE.SphereGeometry(0.14, 10, 8), new THREE.MeshBasicMaterial({ color: 0xffe9a8 }));
    bulb.position.y = 2.7; lamp.add(bulb);
    world.add(at(lamp, x, -0.3, z));
  });
  const cars = [];
  [
    { c: 0xf28c28, lane: 18.6, dir: 1, speed: 5.2, x: -8 },
    { c: 0x3aa79c, lane: 22.4, dir: -1, speed: 4.4, x: 18 },
    { c: 0xf2c94c, lane: 18.6, dir: 1, speed: 3.8, x: 2 },
  ].forEach(d => {
    const c = car(d.c); c.position.set(d.x, -0.28, d.lane); c.rotation.y = d.dir > 0 ? 0 : Math.PI;
    c.userData.drive = d; world.add(c); cars.push(c);
  });
  const parked = car(0x7db4ff); parked.position.set(24.9, -0.28, -14); parked.rotation.y = Math.PI / 2; world.add(parked);

  // ===== gedung kantor =====
  const office = new THREE.Group(); scene.add(office);
  office.add(at(box(38.6, 0.62, 26.6, 0xf2f5f8, { roughness: 0.6 }), 0, 0, 0));
  office.add(at(box(39.2, 0.18, 27.2, 0xc9d3df, { roughness: 0.7 }), 0, -0.2, 0));
  const carpet = (cx, cz, w, d, c) => { const m = box(w, 0.05, d, c, { roughness: 1 }); at(m, cx, 0.34, cz); m.castShadow = false; office.add(m); };
  carpet(-12.5, -8, 10.6, 7.6, 0xcfe2ff);
  carpet(-1.5, -8.5, 8.6, 6.6, 0xe2d4fb);
  carpet(11, -8, 11.6, 7.6, 0xc8edd9);
  carpet(-12.5, 6, 10.6, 9.6, 0xffdfbe);
  carpet(3.5, 5.5, 16.6, 10.6, 0xfcefc6);
  carpet(15.5, 5.5, 5.6, 10.6, 0xd3dde8);

  const glassMat = new THREE.MeshStandardMaterial({ color: 0xbfdcf2, transparent: true, opacity: 0.22, roughness: 0.15, metalness: 0.1, depthWrite: false });
  const frameMat = mat(0xffffff, { roughness: 0.5 });
  function glassWall(x, z, len, horiz = true) {
    const g = new THREE.Group();
    const p = new THREE.Mesh(new THREE.BoxGeometry(horiz ? len : 0.08, 2.3, horiz ? 0.08 : len), glassMat);
    p.position.y = 1.48; g.add(p);
    const rail = new THREE.Mesh(new THREE.BoxGeometry(horiz ? len : 0.14, 0.09, horiz ? 0.14 : len), frameMat);
    rail.position.y = 2.66; g.add(rail);
    [-len / 2, len / 2].forEach(off => {
      const post = new THREE.Mesh(new THREE.BoxGeometry(0.13, 2.7, 0.13), frameMat);
      post.position.set(horiz ? off : 0, 1.35, horiz ? 0 : off); g.add(post);
    });
    g.position.set(x, 0.31, z); office.add(g);
  }
  glassWall(0, -12.8, 38.4); glassWall(0, 12.8, 38.4);
  glassWall(-19, 0, 25.6, false); glassWall(19, 0, 25.6, false);
  glassWall(-6.8, -8, 7.6, false); glassWall(4.8, -8, 7.6, false);
  glassWall(-6.8, 3.4, 9.2); glassWall(12.6, -4.2, 12.6);
  glassWall(12.6, 5.5, 10.6, false); glassWall(-6.8, -4.2, 9.6);

  const board = (txt, x, y, z, ry = 0, w = 2.8) => { const b = textBoard(txt, w); at(b, x, y, z, ry); office.add(b); };
  board('CLAUDE', -12.5, 2.1, -12.6); board('MEETING', -1.5, 2.1, -12.6, 0, 2.4);
  board('CURSOR', 11, 2.1, -12.6); board('OMP', 3.5, 2.2, -4.35, 0, 2.0);
  board('MCP', 15.5, 2.1, 0.35, -Math.PI / 2, 2.0); board('LOUNGE', -12.5, 2.1, 1.05, 0, 2.4);

  const entrance = textBoard('RIZAL AGENT HQ', 5.2, 0.95, '#2f5cff');
  at(entrance, 3.5, 2.25, 12.86); office.add(entrance); tagHotspot(entrance, 'entrance');
  office.add(at(box(4.6, 0.14, 1.6, 0xdde5ee, { roughness: 0.8 }), 3.5, 0.4, 13.6));
  office.add(at(plant(1.05), 0.9, 0.36, 12.2)); office.add(at(plant(1.05), 6.1, 0.36, 12.2));

  function tagHotspot(obj, id) { obj.traverse(o => { o.userData.hotspot = id; }); }

  // Studio Muse
  office.add(at(desk(2.0, 0x4a7dff), -12.5, 0.36, -7.5));
  office.add(at(desk(1.7, 0x7db4ff), -9.6, 0.36, -10.4, Math.PI));
  const shelf = new THREE.Group();
  shelf.add(at(box(2.4, 1.7, 0.3, 0xb08968), 0, 0.85, 0));
  [0xff8a5c, 0x5fd68b, 0x4a7dff, 0xf2c94c, 0xe5484d].forEach((c, i) => shelf.add(at(box(0.24, 0.5 + (i % 3) * 0.12, 0.2, c), -0.9 + i * 0.42, 0.62, 0.17)));
  at(shelf, -16.8, 0.36, -10.5); office.add(shelf);
  office.add(at(plant(1.1), -17, 0.36, -5.2));

  // Meeting
  const mt = new THREE.Group();
  mt.add(at(box(3.8, 0.1, 1.5, 0xb08968), 0, 0.78, 0));
  mt.add(at(box(0.14, 0.76, 1.2, 0x8a6b4f), -1.6, 0.39, 0)); mt.add(at(box(0.14, 0.76, 1.2, 0x8a6b4f), 1.6, 0.39, 0));
  at(mt, -1.5, 0.36, -8.5); office.add(mt);
  [[-2.9, -7.5, Math.PI / 2], [-1.5, -7.5, Math.PI / 2], [-0.1, -7.5, Math.PI / 2],
   [-2.9, -9.5, -Math.PI / 2], [-1.5, -9.5, -Math.PI / 2], [-0.1, -9.5, -Math.PI / 2]]
    .forEach(([x, z, ry], i) => office.add(at(chair([0xb49af2, 0x4a7dff, 0x5fd68b][i % 3]), x, 0.36, z, ry)));
  const chartMesh = at(chartScreen(), -1.5, 1.85, -12.55); office.add(chartMesh);
  tagHotspot(chartMesh, 'chart');

  // Cursor
  office.add(at(desk(2.0, 0x37b58c), 10.5, 0.36, -7.5));
  office.add(at(desk(1.7, 0x67d7b5), 13.8, 0.36, -10.4, Math.PI));
  office.add(at(plant(1.1), 16.2, 0.36, -5.2));

  // Lounge
  const sofa = new THREE.Group();
  sofa.add(at(box(2.6, 0.5, 1.0, 0x4db6ac), 0, 0.32, 0));
  sofa.add(at(box(2.6, 0.75, 0.28, 0x43a79d), 0, 0.62, -0.4));
  sofa.add(at(box(0.3, 0.55, 1.0, 0x43a79d), -1.3, 0.45, 0)); sofa.add(at(box(0.3, 0.55, 1.0, 0x43a79d), 1.3, 0.45, 0));
  at(sofa, -12.5, 0.36, 3.2); office.add(sofa); tagHotspot(sofa, 'sofa');
  office.add(at(box(1.2, 0.08, 0.6, 0xffffff), -12.5, 0.62, 4.6));
  const tt = new THREE.Group();
  tt.add(at(box(2.5, 0.07, 1.35, 0x2f9e8f), 0, 0.76, 0));
  tt.add(at(box(2.3, 0.03, 0.03, 0xffffff), 0, 0.83, 0));
  tt.add(at(box(0.08, 0.74, 1.1, 0x8fa0b5), -1.0, 0.38, 0)); tt.add(at(box(0.08, 0.74, 1.1, 0x8fa0b5), 1.0, 0.38, 0));
  at(tt, -12.5, 0.36, 8.6); office.add(tt); tagHotspot(tt, 'pingpong');
  const bb1 = sph(0.5, 0xff8a5c); bb1.scale.y = 0.62; at(bb1, -16.6, 0.62, 6.4); office.add(bb1);
  const bb2 = sph(0.5, 0xf276b0); bb2.scale.y = 0.62; at(bb2, -8.4, 0.62, 6.9); office.add(bb2);
  office.add(at(plant(1.2), -17.2, 0.36, 10.6));

  // Hall OMP + worker pods
  office.add(at(desk(2.7, 0xf2a541, 0xffd9a0), 3.5, 0.36, 1.6));
  [-2.5, 0.8, 4.1, 7.4].forEach((x, i) => office.add(at(desk(1.5, [0x5fd68b, 0x4a7dff, 0xf2c94c, 0xb49af2][i % 4]), x, 0.36, 5.6)));
  [-2.5, 0.8, 4.1, 7.4].forEach((x, i) => office.add(at(desk(1.5, [0xf276b0, 0x3ec6b8, 0xff8a5c, 0x7db4ff][i % 4]), x, 0.36, 9.2)));
  office.add(at(plant(1), -4.2, 0.36, 1.2)); office.add(at(plant(1), 11.2, 0.36, 1.4));

  // Rak server MCP
  const rackGroups = [];
  [1.8, 5.5, 9.2].forEach((z, ri) => {
    const rack = new THREE.Group(); rackGroups.push(rack);
    rack.add(at(box(0.95, 2.3, 0.85, 0x2c3644), 0, 1.15, 0));
    for (let i = 0; i < 5; i++) {
      const dot = new THREE.Mesh(new THREE.PlaneGeometry(0.6, 0.07), new THREE.MeshBasicMaterial({ color: [0x57e6a3, 0xffb454, 0x7db4ff][(i + ri) % 3] }));
      dot.position.set(-0.48, 0.4 + i * 0.42, 0); dot.rotation.y = -Math.PI / 2; rack.add(dot);
    }
    at(rack, 17.6, 0.36, z); office.add(rack); tagHotspot(rack, 'rack');
  });

  // ===== meeting =====
  const MEETING_SPOTS = [[-2.9, -7.15, 0], [-1.5, -7.15, 0], [-0.1, -7.15, 0],
    [-2.9, -9.85, Math.PI], [-1.5, -9.85, Math.PI], [-0.1, -9.85, Math.PI],
    [-3.7, -8.5, Math.PI / 2], [0.7, -8.5, -Math.PI / 2]];
  const meeting = { active: false, until: 0, invited: new Set(), order: new Map() };

  // ===== agent =====
  const entries = new Map();
  const pickables = [];
  let seedCounter = 0;

  function slotFor(a, idx) {
    if (a.id === 'omp') return SLOTS.omp[0];
    if (a.id === 'Muse') return SLOTS.Muse[idx.Muse++ % SLOTS.Muse.length];
    if (a.id === 'cursor') return SLOTS.cursor[idx.cursor++ % SLOTS.cursor.length];
    if (a.id === 'mcp') return SLOTS.mcp[0];
    if (a.id === 'omp-workers-agg') return SLOTS.agg[0];
    if (a.kind === 'worker') return SLOTS.workers[idx.workers++] || SLOTS.overflow[idx.overflow++ % SLOTS.overflow.length];
    if (a.kind === 'utama') return SLOTS.extra[idx.extra++ % SLOTS.extra.length];
    return SLOTS.overflow[idx.overflow++ % SLOTS.overflow.length];
  }

  function update(agents) {
    const idx = { Muse: 0, cursor: 0, workers: 0, extra: 0, overflow: 0 };
    const seen = new Set();
    for (const a of agents) {
      seen.add(a.id);
      let e = entries.get(a.id);
      if (!e) {
        const isRobot = a.kind === 'worker' || a.kind === 'mcp';
        const colorHex = parseInt((a.color || '#4a7dff').replace('#', ''), 16);
        const group = isRobot ? makeRobot(colorHex) : makeHuman(colorHex, seedCounter++);
        const ring = new THREE.Mesh(new THREE.RingGeometry(0.52, 0.72, 28),
          new THREE.MeshBasicMaterial({ color: 0x9aa7ba, transparent: true, opacity: 0.95, side: THREE.DoubleSide }));
        ring.rotation.x = -Math.PI / 2; ring.position.y = 0.375;
        scene.add(group); scene.add(ring);
        const chip = document.createElement('div');
        chip.className = 'alabel';
        chip.title = a.name;
        chip.onclick = () => onSelect && onSelect(a.id);
        if (labelsEl) labelsEl.appendChild(chip);
        e = { group, ring, chip, agent: a, baseY: 0.36, slotSpot: null, placed: false, jumpUntil: 0 };
        group.traverse(o => { o.userData.agentId = a.id; });
        entries.set(a.id, e);
        pickables.push(group);
      }
      e.agent = a;
      e.slotSpot = slotFor(a, idx);
      if (!e.placed) {
        e.group.position.set(e.slotSpot[0], e.baseY, e.slotSpot[1]);
        e.group.rotation.y = e.slotSpot[2];
        e.placed = true;
      }
      e.ring.material.color.setHex(STATUS_COLORS[a.status] ?? 0x9aa7ba);
      const dot = STATUS_CSS[a.status] || '#9aa7ba';
      const short = a.name.replace(/^Worker · /, '');
      const actTxt = (a.status === 'bekerja' || a.status === 'berpikir') && a.activity
        ? a.activity.replace(/^[A-Za-z✨🤖🧠🖱️⌨️🧩🔌 ]+? (sedang|mulai|membuka|menjalankan|menulis|memperbaiki|meneliti|mengubah|mencari|membaca)/, (m, v) => v[0].toUpperCase() + v.slice(1))
        : '';
      const actShort = actTxt.length > 46 ? actTxt.slice(0, 46) + '…' : actTxt;
      const actHtml = actShort ? `<small class="act">${actShort.replace(/&/g, '&amp;').replace(/</g, '&lt;')}</small>` : '';
      e.chip.innerHTML = `<i style="background:${dot}"></i>${a.emoji || ''} ${short}<small>${(window.HQ_STATUS && window.HQ_STATUS[a.status]) || STATUS_LABEL[a.status] || a.status}</small>${actHtml}`;
      e.chip.style.opacity = a.status === 'offline' ? 0.55 : 1;
    }
    for (const [id, e] of entries) {
      if (!seen.has(id)) {
        scene.remove(e.group); scene.remove(e.ring); e.chip.remove();
        entries.delete(id);
        const pi = pickables.indexOf(e.group); if (pi >= 0) pickables.splice(pi, 1);
      }
    }
  }

  // ===== kamera =====
  let desired = { x: 0, z: 0, zoom: 0.74 };
  function focusAgent(id) {
    const e = entries.get(id);
    if (e) desired = { x: e.group.position.x, z: e.group.position.z, zoom: 1.75 };
  }
  function preset(name, agents) {
    if (name === 'iso') { camDir = ISO_DIR.clone(); desired = { x: 0, z: 0, zoom: 0.74 }; return; }
    if (name === 'atas') { camDir = new THREE.Vector3(0.001, 1, 0.35).normalize(); desired = { x: 0, z: 0, zoom: 0.8 }; return; }
    if (name === 'alert') {
      const bad = (agents || []).find(a => a.status === 'menunggu-rizal' || a.status === 'error');
      if (bad) { focusAgent(bad.id); return; }
      desired = { x: 0, z: 0, zoom: 0.74 }; return;
    }
    const z = ZONES[name] || ZONES.semua;
    desired = { x: z.t[0], z: z.t[1], zoom: z.zoom };
  }

  function callMeeting() {
    if (meeting.active) { meeting.active = false; onMeetingChange && onMeetingChange(false); return false; }
    meeting.invited = new Set(); meeting.order = new Map();
    let i = 0;
    for (const e of entries.values()) if (e.agent.kind === 'utama') { meeting.invited.add(e.agent.id); meeting.order.set(e.agent.id, i++); }
    if (!meeting.invited.size) return false;
    meeting.active = true;
    meeting.until = simT + 30;
    onMeetingChange && onMeetingChange(true);
    return true;
  }

  // ===== interaksi =====
  const ray = new THREE.Raycaster();
  const ptr = new THREE.Vector2();
  let downPos = null;
  const hotspotObjs = [];
  scene.traverse(o => { if (o.userData.hotspot) hotspotObjs.push(o); });
  function pick(ev) {
    const r = renderer.domElement.getBoundingClientRect();
    ptr.x = ((ev.clientX - r.left) / r.width) * 2 - 1;
    ptr.y = -((ev.clientY - r.top) / r.height) * 2 + 1;
    ray.setFromCamera(ptr, camera);
    const hits = ray.intersectObjects(pickables, true);
    if (hits.length) return { agent: hits[0].object.userData.agentId };
    const hh = ray.intersectObjects(hotspotObjs, false);
    if (hh.length) return { hotspot: hh[0].object.userData.hotspot };
    return null;
  }
  function nearestEntryJump(x, z) {
    let best = null, bd = 1e9;
    for (const e of entries.values()) {
      const d = Math.hypot(e.group.position.x - x, e.group.position.z - z);
      if (d < bd) { bd = d; best = e; }
    }
    if (best) best.jumpUntil = simT + 0.6;
  }
  renderer.domElement.addEventListener('pointerdown', ev => { downPos = [ev.clientX, ev.clientY]; });
  renderer.domElement.addEventListener('pointerup', ev => {
    if (!downPos) return;
    const moved = Math.hypot(ev.clientX - downPos[0], ev.clientY - downPos[1]);
    downPos = null;
    if (moved > 6) return;
    const hit = pick(ev);
    if (!hit) return;
    if (hit.agent && onSelect) onSelect(hit.agent);
    else if (hit.hotspot) {
      if (hit.hotspot === 'pingpong') nearestEntryJump(-12.5, 8.6);
      if (hit.hotspot === 'sofa') nearestEntryJump(-12.5, 3.2);
      if (onInteract) onInteract(hit.hotspot, ev);
    }
  });
  renderer.domElement.addEventListener('pointermove', ev => {
    const hit = pick(ev);
    renderer.domElement.style.cursor = hit ? 'pointer' : 'grab';
    if (onHover) onHover(hit && hit.agent ? hit.agent : null, ev);
  });

  function resize() {
    const w = stageEl.clientWidth || innerWidth, h = stageEl.clientHeight || innerHeight;
    renderer.setSize(w, h);
    const span = 28.5;
    camera.left = -span * w / h / 2; camera.right = span * w / h / 2;
    camera.top = span / 2; camera.bottom = -span / 2;
    camera.updateProjectionMatrix();
  }
  new ResizeObserver(resize).observe(stageEl);
  resize();

  const v3 = new THREE.Vector3();
  const clockT = new THREE.Clock();
  let lastT = 0, frames = 0, lastDt = 0;
  // --- kendali ala The Sims ---
  let simT = 0, speedMul = 1, simPaused = false, orbitOn = false, followId = null, plumbobId = null;
  const plumbob = new THREE.Mesh(
    new THREE.OctahedronGeometry(0.17),
    new THREE.MeshStandardMaterial({ color: 0x2bff88, emissive: 0x12b76a, emissiveIntensity: 0.9, roughness: 0.25, metalness: 0.1 })
  );
  plumbob.visible = false;
  scene.add(plumbob);
  const AMBIENCES = {
    day: { bg: 0xe9edf2, hemi: 0.88, sun: 2.15, sunColor: 0xffffff },
    sunset: { bg: 0xf7ddba, hemi: 0.62, sun: 1.35, sunColor: 0xffb26b },
    night: { bg: 0x232f4d, hemi: 0.34, sun: 0.5, sunColor: 0x9db8ff },
  };
  function setAmbience(name) {
    const a = AMBIENCES[name] || AMBIENCES.day;
    scene.background = new THREE.Color(a.bg);
    hemi.intensity = a.hemi;
    sun.intensity = a.sun;
    sun.color.setHex(a.sunColor);
  }
  function angLerp(cur, target, k) {
    let d = (target - cur) % (Math.PI * 2);
    if (d > Math.PI) d -= Math.PI * 2;
    if (d < -Math.PI) d += Math.PI * 2;
    return cur + d * k;
  }
  function animate() {
    requestAnimationFrame(animate);
    const tRaw = clockT.getElapsedTime();
    const rawDt = Math.min(tRaw - lastT, 0.1); lastT = tRaw; frames++; lastDt = rawDt;
    const dt = simPaused ? 0 : rawDt * speedMul;
    simT += dt;
    const t = simT;
    if (orbitOn && !simPaused) camDir.applyAxisAngle(new THREE.Vector3(0, 1, 0), dt * 0.07);
    if (followId && entries.has(followId)) {
      const fe = entries.get(followId);
      desired.x = fe.group.position.x; desired.z = fe.group.position.z;
    }
    if (plumbobId && entries.has(plumbobId)) {
      const pe = entries.get(plumbobId);
      plumbob.visible = true;
      plumbob.position.set(pe.group.position.x, pe.group.position.y + 2.75 + Math.sin(t * 3.1) * 0.09, pe.group.position.z);
      plumbob.rotation.y = t * 2.6;
    } else plumbob.visible = false;
    if (meeting.active && t > meeting.until) { meeting.active = false; onMeetingChange && onMeetingChange(false); }
    const prevTarget = controls.target.clone();
    controls.target.lerp(v3.set(desired.x, 0, desired.z), 0.07);
    camera.position.add(controls.target.clone().sub(prevTarget));
    const want = controls.target.clone().add(camDir.clone().multiplyScalar(70));
    camera.position.lerp(want, 0.045);
    camera.zoom += (desired.zoom - camera.zoom) * 0.07;
    camera.updateProjectionMatrix();
    controls.update();
    for (const c of cars) {
      const d = c.userData.drive;
      c.position.x += d.dir * d.speed * dt * 0.96;
      if (c.position.x > 55) c.position.x = -55;
      if (c.position.x < -55) c.position.x = 55;
      c.userData.wheels.forEach(w => { w.rotation.z -= d.dir * 0.12; });
    }
    for (const e of entries.values()) {
      const a = e.agent, p = e.group.userData.parts || {};
      const ph = e.group.position.x * 1.7 + e.group.position.z;
      // --- gerak menuju slot / kursi meeting ---
      const spot = (meeting.active && meeting.invited.has(a.id))
        ? MEETING_SPOTS[meeting.order.get(a.id) % MEETING_SPOTS.length]
        : (e.slotSpot || [e.group.position.x, e.group.position.z, e.group.rotation.y]);
      const dx = spot[0] - e.group.position.x, dz = spot[1] - e.group.position.z;
      const dist = Math.hypot(dx, dz);
      let walking = false, faceT = spot[2];
      if (dist > 0.04) {
        walking = true;
        const step = Math.min(dist, dt * 2.6);
        e.group.position.x += dx / dist * step;
        e.group.position.z += dz / dist * step;
        faceT = Math.atan2(dx, dz);
      }
      e.group.rotation.y = angLerp(e.group.rotation.y, faceT, walking ? 0.25 : 0.12);
      e.ring.position.set(e.group.position.x, 0.375, e.group.position.z);
      let jumpY = 0;
      if (e.jumpUntil && t < e.jumpUntil) jumpY = Math.sin((1 - (e.jumpUntil - t) / 0.6) * Math.PI) * 0.3;
      if (walking) {
        e.group.position.y = e.baseY + Math.abs(Math.sin(t * 9 + ph)) * 0.05 + jumpY;
        if (p.armL) p.armL.rotation.x = Math.sin(t * 9 + ph) * 0.55;
        if (p.armR) p.armR.rotation.x = -Math.sin(t * 9 + ph) * 0.55;
      } else if (a.status === 'bekerja') {
        e.group.position.y = e.baseY + Math.abs(Math.sin(t * 3.2 + ph)) * 0.035 + jumpY;
        if (p.armL) p.armL.rotation.x = Math.sin(t * 7 + ph) * 0.35 - 0.5;
        if (p.armR) p.armR.rotation.x = Math.cos(t * 7 + ph) * 0.35 - 0.5;
        if (p.head) p.head.rotation.y = Math.sin(t * 0.9 + ph) * 0.16;
      } else if (a.status === 'berpikir') {
        if (p.head) { p.head.rotation.z = Math.sin(t * 1.1 + ph) * 0.1; p.head.rotation.y = Math.sin(t * 0.5 + ph) * 0.3; }
        e.group.position.y = e.baseY + jumpY;
      } else if (a.status === 'menunggu-rizal' || a.status === 'error') {
        e.group.position.y = e.baseY + Math.abs(Math.sin(t * 4.5 + ph)) * 0.06 + jumpY;
        if (p.head) p.head.rotation.y = Math.sin(t * 2 + ph) * 0.2;
      } else {
        e.group.position.y = e.baseY + Math.sin(t * 1.3 + ph) * 0.012 + jumpY;
        if (p.armL) p.armL.rotation.x = 0; if (p.armR) p.armR.rotation.x = 0;
        if (p.head) { p.head.rotation.y = 0; p.head.rotation.z = 0; }
      }
      e.ring.scale.setScalar((a.status === 'bekerja' || a.status === 'menunggu-rizal' || a.status === 'error')
        ? 1 + Math.sin(t * 3.4 + ph) * 0.1 : 1);
      if (e.chip && labelsEl) {
        v3.set(e.group.position.x, e.group.position.y + 2.1, e.group.position.z).project(camera);
        const w = stageEl.clientWidth, h = stageEl.clientHeight;
        if (v3.z > 1 || v3.x < -1.05 || v3.x > 1.05 || v3.y < -1.05 || v3.y > 1.05) e.chip.style.display = 'none';
        else {
          e.chip.style.display = 'block';
          e.chip.style.left = ((v3.x * 0.5 + 0.5) * w) + 'px';
          e.chip.style.top = ((-v3.y * 0.5 + 0.5) * h) + 'px';
        }
      }
    }
    renderer.render(scene, camera);
  }
  animate();

  return { update, preset, focusAgent, callMeeting,
    setPaused: v => { simPaused = !!v; },
    setSpeed: v => { speedMul = v || 1; },
    setAmbience, setOrbit: v => { orbitOn = !!v; },
    setFollow: id => { followId = id || null; },
    setPlumbob: id => { plumbobId = id || null; },
    debug: () => ({ active: meeting.active, frames, t: +clockT.getElapsedTime().toFixed(1), lastDt, invited: [...meeting.invited],
      pos: [...entries.values()].map(e => [e.agent.id, +e.group.position.x.toFixed(1), +e.group.position.z.toFixed(1), e.agent.kind]) }) };
}
