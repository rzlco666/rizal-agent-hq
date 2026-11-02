// Kantor 3D Rizal Agent HQ — Three.js.
// Agent utama = manusia low-poly; worker & MCP = robot kecil (keputusan 3D-C).
// Animasi mengikuti 6 status seragam. Klik karakter -> onSelect(agentId).

import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';

export const STATUS_COLORS = {
  'bekerja': 0x3ddc97, 'berpikir': 0x7c9eff, 'menunggu-rizal': 0xffb454,
  'santai': 0x5b6b8c, 'offline': 0x3a4358, 'error': 0xff6b7a,
};
const PALETTE = [0x3ddc97, 0x7c9eff, 0xffb454, 0xff8fab, 0x64dfdf, 0xc39bff, 0xffd166, 0x8ce99a];

function colorFor(id) {
  let h = 0;
  for (const c of id) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return PALETTE[h % PALETTE.length];
}

function textSprite(draw, w = 512, h = 128, scale = [4.4, 1.1]) {
  const cv = document.createElement('canvas');
  cv.width = w; cv.height = h;
  draw(cv.getContext('2d'), w, h);
  const tex = new THREE.CanvasTexture(cv);
  tex.anisotropy = 4;
  const sp = new THREE.Sprite(new THREE.SpriteMaterial({ map: tex, transparent: true, depthWrite: false }));
  sp.scale.set(scale[0], scale[1], 1);
  return sp;
}

function nameSprite(agent) {
  const small = agent.kind !== 'utama';
  return textSprite((g, w, h) => {
    g.fillStyle = 'rgba(10,14,26,0.78)';
    g.beginPath(); g.roundRect(56, 18, w - 112, 62, 16); g.fill();
    g.font = '600 34px system-ui, sans-serif';
    g.textAlign = 'center'; g.textBaseline = 'middle';
    g.fillStyle = '#e8edf7';
    g.fillText(`${agent.emoji} ${agent.name}`.slice(0, 26), w / 2, 50);
  }, 512, 128, small ? [3.1, 0.78] : [4.4, 1.1]);
}

function bubbleSprite(text) {
  return textSprite((g, w, h) => {
    g.fillStyle = 'rgba(16,22,40,0.92)';
    g.beginPath(); g.roundRect(8, 8, w - 16, h - 30, 18); g.fill();
    g.beginPath(); g.moveTo(w / 2 - 12, h - 24); g.lineTo(w / 2 + 12, h - 24); g.lineTo(w / 2, h - 4); g.fill();
    g.font = '27px system-ui, sans-serif';
    g.fillStyle = '#dbe4f5'; g.textAlign = 'center';
    const words = text.split(' ');
    const lines = []; let cur = '';
    for (const wd of words) {
      if ((cur + ' ' + wd).length > 34) { lines.push(cur); cur = wd; if (lines.length === 3) break; }
      else cur = cur ? cur + ' ' + wd : wd;
    }
    if (lines.length < 3 && cur) lines.push(cur);
    lines.slice(0, 3).forEach((ln, i) => g.fillText(ln, w / 2, 40 + i * 32));
  }, 512, 160, [4.6, 1.44]);
}

function makeHuman(color) {
  const g = new THREE.Group();
  const mat = new THREE.MeshStandardMaterial({ color, roughness: 0.65 });
  const body = new THREE.Mesh(new THREE.CapsuleGeometry(0.34, 0.55, 6, 14), mat);
  body.position.y = 0.95; body.name = 'body';
  const head = new THREE.Mesh(new THREE.SphereGeometry(0.27, 20, 16),
    new THREE.MeshStandardMaterial({ color: 0xf2c79b, roughness: 0.8 }));
  head.position.y = 1.78; head.name = 'head';
  const hair = new THREE.Mesh(new THREE.SphereGeometry(0.28, 20, 12, 0, Math.PI * 2, 0, 1.25),
    new THREE.MeshStandardMaterial({ color: 0x2b2118, roughness: 0.9 }));
  hair.position.y = 1.82;
  g.add(body, head, hair);
  g.userData.body = body; g.userData.head = head;
  return g;
}

function makeRobot(color) {
  const g = new THREE.Group();
  const mat = new THREE.MeshStandardMaterial({ color, roughness: 0.4, metalness: 0.35 });
  const body = new THREE.Mesh(new THREE.BoxGeometry(0.62, 0.72, 0.44), mat);
  body.position.y = 0.62; body.name = 'body';
  const head = new THREE.Mesh(new THREE.BoxGeometry(0.56, 0.42, 0.46),
    new THREE.MeshStandardMaterial({ color: 0xdfe6f2, roughness: 0.35, metalness: 0.2 }));
  head.position.y = 1.22; head.name = 'head';
  const eyeMat = new THREE.MeshBasicMaterial({ color: 0x0b1020 });
  for (const dx of [-0.13, 0.13]) {
    const eye = new THREE.Mesh(new THREE.SphereGeometry(0.05, 10, 8), eyeMat);
    eye.position.set(dx, 1.24, 0.24); g.add(eye);
  }
  const ant = new THREE.Mesh(new THREE.CylinderGeometry(0.03, 0.03, 0.34), mat);
  ant.position.y = 1.58;
  const tip = new THREE.Mesh(new THREE.SphereGeometry(0.07, 10, 8),
    new THREE.MeshBasicMaterial({ color: 0xffffff }));
  tip.position.y = 1.78; tip.name = 'tip';
  g.add(body, head, ant, tip);
  g.userData.body = body; g.userData.head = head; g.userData.tip = tip;
  return g;
}

function makeDesk(w, color) {
  const g = new THREE.Group();
  const top = new THREE.Mesh(new THREE.BoxGeometry(w, 0.09, 0.95),
    new THREE.MeshStandardMaterial({ color: 0x3a2e20, roughness: 0.8 }));
  top.position.y = 0.74;
  const legL = new THREE.Mesh(new THREE.BoxGeometry(0.08, 0.74, 0.8),
    new THREE.MeshStandardMaterial({ color: 0x241b12 }));
  legL.position.set(-w / 2 + 0.08, 0.37, 0);
  const legR = legL.clone(); legR.position.x = w / 2 - 0.08;
  const mon = new THREE.Mesh(new THREE.BoxGeometry(0.72, 0.46, 0.04),
    new THREE.MeshStandardMaterial({ color: 0x11182b, emissive: color, emissiveIntensity: 0.55 }));
  mon.position.set(0.18, 1.06, -0.22); mon.rotation.y = -0.12; mon.name = 'monitor';
  const kb = new THREE.Mesh(new THREE.BoxGeometry(0.55, 0.03, 0.2),
    new THREE.MeshStandardMaterial({ color: 0x1a2338 }));
  kb.position.set(0, 0.8, 0.02);
  g.add(top, legL, legR, mon, kb);
  g.userData.monitor = mon;
  return g;
}

export function initOffice(stage, { onSelect, onHover } = {}) {
  const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
  renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
  renderer.shadowMap.enabled = false;
  stage.appendChild(renderer.domElement);

  const scene = new THREE.Scene();
  scene.fog = new THREE.Fog(0x0b1020, 30, 60);
  const camera = new THREE.PerspectiveCamera(46, 1, 0.1, 200);
  camera.position.set(0, 12.5, 19.5);
  const controls = new OrbitControls(camera, renderer.domElement);
  controls.target.set(0, 0.6, -1);
  controls.maxPolarAngle = Math.PI * 0.49;
  controls.minDistance = 5; controls.maxDistance = 38;

  scene.add(new THREE.HemisphereLight(0xbdd3ff, 0x1a1408, 0.95));
  const dir = new THREE.DirectionalLight(0xfff2d8, 1.5);
  dir.position.set(7, 13, 6);
  scene.add(dir);

  const floor = new THREE.Mesh(new THREE.CircleGeometry(17.5, 56),
    new THREE.MeshStandardMaterial({ color: 0x141d33, roughness: 0.95 }));
  floor.rotation.x = -Math.PI / 2;
  scene.add(floor);
  const rug = new THREE.Mesh(new THREE.CircleGeometry(11.5, 48),
    new THREE.MeshStandardMaterial({ color: 0x1a2745, roughness: 1 }));
  rug.rotation.x = -Math.PI / 2; rug.position.y = 0.01;
  scene.add(rug);
  // Papan tulis ringkasan di belakang.
  const board = new THREE.Mesh(new THREE.BoxGeometry(9.5, 2.7, 0.16),
    new THREE.MeshStandardMaterial({ color: 0x0e2a22, roughness: 0.7 }));
  board.position.set(0, 3.4, -10.6);
  scene.add(board);
  const boardFrame = new THREE.Mesh(new THREE.BoxGeometry(9.9, 3.1, 0.1),
    new THREE.MeshStandardMaterial({ color: 0x5a4028 }));
  boardFrame.position.set(0, 3.4, -10.72);
  scene.add(boardFrame);

  const units = new Map(); // id -> {group, char, ring, nameSp, bubble, agent, slot}
  const ray = new THREE.Raycaster();
  const ptr = new THREE.Vector2();
  let downAt = null;
  let camGoal = null;

  function slotFor(index, total, kind) {
    // Utama: baris depan melengkung; worker: grid belakang; mcp: sisi kanan.
    if (kind === 'mcp') return { x: 8.6, z: -2 + index * 2.2, rotY: -Math.PI / 2.4 };
    if (kind === 'worker') {
      const col = index % 5, row = Math.floor(index / 5);
      return { x: -7.6 + col * 3.8, z: -4.4 - row * 3.4, rotY: 0 };
    }
    const cols = Math.min(total, 4);
    const col = index % cols, row = Math.floor(index / cols);
    return { x: (col - (cols - 1) / 2) * 4.6, z: 3.0 - row * 3.8, rotY: 0 };
  }

  function ensureUnit(agent) {
    let u = units.get(agent.id);
    if (u) return u;
    const color = agent.parent ? colorFor(agent.parent) : colorFor(agent.id);
    const grp = new THREE.Group();
    const isRobot = agent.kind !== 'utama';
    const char = isRobot ? makeRobot(color) : makeHuman(color);
    char.position.z = -0.58; // duduk di sisi jauh meja, menghadap kamera
    const desk = makeDesk(isRobot ? 1.5 : 2.3, STATUS_COLORS[agent.status] ?? 0x3ddc97);
    const ring = new THREE.Mesh(new THREE.RingGeometry(1.05, 1.28, 40),
      new THREE.MeshBasicMaterial({ color: STATUS_COLORS[agent.status] ?? 0xffffff, transparent: true, opacity: 0.85, side: THREE.DoubleSide }));
    ring.rotation.x = -Math.PI / 2; ring.position.y = 0.03;
    const nameSp = nameSprite(agent); nameSp.position.y = agent.kind !== 'utama' ? 2.45 : 2.85;
    const bang = textSprite((g, w, h) => {
      g.font = '900 92px system-ui'; g.textAlign = 'center';
      g.fillStyle = '#ffb454'; g.fillText('!', w / 2, 96);
    }, 128, 128, [0.8, 0.8]);
    bang.position.y = 3.35; bang.visible = false; bang.name = 'bang';
    grp.add(desk, char, ring, nameSp, bang);
    grp.userData.agentId = agent.id;
    scene.add(grp);
    u = { group: grp, char, desk, ring, nameSp, bang, bubble: null, bubbleText: '', agent };
    units.set(agent.id, u);
    return u;
  }

  function update(agents) {
    const utama = agents.filter(a => a.kind === 'utama');
    const workers = agents.filter(a => a.kind === 'worker');
    const mcps = agents.filter(a => a.kind === 'mcp');
    const seen = new Set();
    const place = (list, kind) => list.forEach((a, i) => {
      seen.add(a.id);
      const u = ensureUnit(a);
      u.agent = a;
      const s = slotFor(i, list.length, kind);
      u.slot = s;
      if (!u.placed) { u.group.position.set(s.x, 0, s.z); u.group.rotation.y = s.rotY; u.placed = true; }
      const sc = STATUS_COLORS[a.status] ?? 0xffffff;
      u.ring.material.color.setHex(sc);
      u.desk.userData.monitor.material.emissive.setHex(sc);
      if (u.char.userData.tip) u.char.userData.tip.material.color.setHex(sc);
      u.bang.visible = a.status === 'menunggu-rizal';
      const wantBubble = a.activity && (a.status === 'bekerja' || a.status === 'berpikir' || a.status === 'menunggu-rizal' || a.status === 'error');
      if (wantBubble && u.bubbleText !== a.activity) {
        if (u.bubble) { u.group.remove(u.bubble); u.bubble.material.map.dispose(); }
        u.bubble = bubbleSprite(a.activity);
        u.bubble.position.y = 4.15;
        u.group.add(u.bubble);
        u.bubbleText = a.activity;
      } else if (!wantBubble && u.bubble) {
        u.group.remove(u.bubble); u.bubble = null; u.bubbleText = '';
      }
      const grey = a.status === 'offline';
      u.char.traverse(o => { if (o.material && 'color' in o.material && o.name !== 'tip') o.material.color.lerp(new THREE.Color(grey ? 0x4a5468 : o.material.color.getHex()), 0.2); });
    });
    place(utama, 'utama'); place(workers, 'worker'); place(mcps, 'mcp');
    for (const [id, u] of units) {
      if (!seen.has(id)) { scene.remove(u.group); units.delete(id); }
    }
  }

  function pick(ev) {
    const r = renderer.domElement.getBoundingClientRect();
    ptr.x = ((ev.clientX - r.left) / r.width) * 2 - 1;
    ptr.y = -((ev.clientY - r.top) / r.height) * 2 + 1;
    ray.setFromCamera(ptr, camera);
    const hits = ray.intersectObjects(scene.children, true);
    for (const h of hits) {
      let o = h.object;
      while (o && !o.userData.agentId) o = o.parent;
      if (o) return o.userData.agentId;
    }
    return null;
  }
  renderer.domElement.addEventListener('pointerdown', e => { downAt = [e.clientX, e.clientY]; });
  renderer.domElement.addEventListener('pointerup', e => {
    if (!downAt) return;
    const moved = Math.hypot(e.clientX - downAt[0], e.clientY - downAt[1]);
    downAt = null;
    if (moved > 6) return;
    const id = pick(e);
    if (id && onSelect) onSelect(id);
  });
  renderer.domElement.addEventListener('pointermove', e => {
    if (onHover) onHover(pick(e), e);
  });

  function flyTo(pos, tgt) { camGoal = { pos: new THREE.Vector3(...pos), tgt: new THREE.Vector3(...tgt) }; }
  function preset(name, agents) {
    if (name === 'depan') flyTo([0, 12.5, 19.5], [0, 0.6, -1]);
    else if (name === 'atas') flyTo([0, 26, 4.5], [0, 0, -1]);
    else if (name === 'alert') {
      const a = (agents || []).find(x => x.status === 'menunggu-rizal') || (agents || []).find(x => x.active_alerts > 0);
      if (a) focusAgent(a.id); else flyTo([0, 12.5, 19.5], [0, 0.6, -1]);
    }
  }
  function focusAgent(id) {
    const u = units.get(id);
    if (!u) return;
    const p = u.group.position;
    flyTo([p.x + 2.6, 3.6, p.z + 5.2], [p.x, 1.1, p.z]);
  }

  function resize() {
    const w = stage.clientWidth, h = stage.clientHeight;
    if (!w || !h) return;
    renderer.setSize(w, h);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
  }
  new ResizeObserver(resize).observe(stage);
  resize();

  const clock = new THREE.Clock();
  renderer.setAnimationLoop(() => {
    const t = clock.getElapsedTime();
    for (const u of units.values()) {
      const a = u.agent; if (!a || !u.slot) continue;
      u.group.position.x += (u.slot.x - u.group.position.x) * 0.06;
      u.group.position.z += (u.slot.z - u.group.position.z) * 0.06;
      const body = u.char.userData.body, head = u.char.userData.head;
      if (a.status === 'bekerja') {
        body.position.y = (u.char.userData.bodyBaseY ?? (body.position.y)) + Math.abs(Math.sin(t * 7)) * 0.05;
        head.rotation.y = Math.sin(t * 2.2) * 0.14;
      } else if (a.status === 'berpikir') {
        head.rotation.z = Math.sin(t * 1.4) * 0.12;
        head.rotation.y = Math.sin(t * 0.7) * 0.3;
      } else if (a.status === 'menunggu-rizal') {
        u.bang.position.y = 3.35 + Math.abs(Math.sin(t * 3.4)) * 0.22;
        const s = 1 + Math.sin(t * 3.4) * 0.12;
        u.ring.scale.set(s, s, 1);
      } else if (a.status === 'santai') {
        u.char.rotation.z = Math.sin(t * 0.9) * 0.03;
      } else if (a.status === 'error') {
        u.ring.material.opacity = 0.45 + (Math.sin(t * 6) > 0 ? 0.5 : 0);
      }
      if (a.status !== 'menunggu-rizal') u.ring.scale.set(1, 1, 1);
      if (a.status !== 'error') u.ring.material.opacity = 0.85;
    }
    if (camGoal) {
      camera.position.lerp(camGoal.pos, 0.07);
      controls.target.lerp(camGoal.tgt, 0.09);
      if (camera.position.distanceTo(camGoal.pos) < 0.08) camGoal = null;
    }
    controls.update();
    renderer.render(scene, camera);
  });

  return { update, preset, focusAgent };
}
