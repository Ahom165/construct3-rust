#!/usr/bin/env python3
"""Génère assets/sample.c3p — projet de test au format multi-fichiers C3.
Contient : joueur (Platform), sols (Solid), pièces (Sine), ennemis (Bullet), HUD texte,
events : HUD, collision pièce -> score, collision ennemi -> restart."""

import json, zipfile, struct, zlib, os

def png(w, h, rgba_fn):
    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    raw = b""
    for y in range(h):
        raw += b"\x00"
        for x in range(w):
            raw += bytes(rgba_fn(x, y))
    return (b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw))
            + chunk(b"IEND", b""))

def player_px(x, y):
    dx, dy = x - 32, y - 32
    inside = (dx*dx) / (26*26) + (dy*dy) / (30*30) <= 1.0
    visor = -8 <= dy <= -2 and -14 <= dx <= 6
    if visor:
        return (230, 240, 255, 255)
    if inside:
        return (70, 130, 240, 255)
    return (0, 0, 0, 0)

def enemy_px(x, y):
    dx, dy = x - 16, y - 16
    if abs(dx) < 12 and abs(dy) < 12:
        if abs(dx) < 5 and abs(dy) < 5:
            return (255, 255, 255, 255)
        return (220, 60, 50, 255)
    return (0, 0, 0, 0)

def coin_px(x, y):
    dx, dy = x - 16, y - 16
    r2 = dx*dx + dy*dy
    if r2 < 14*14:
        if r2 < 8*8:
            return (255, 230, 120, 255)
        return (240, 180, 40, 255)
    return (0, 0, 0, 0)

def tile_px(x, y):
    edge = x < 2 or y < 2
    if edge:
        return (60, 45, 35, 255)
    return (120, 90, 60, 255)

def bg_px(x, y):
    if (x // 16 + y // 16) % 2 == 0:
        return (38, 42, 52, 255)
    return (34, 38, 48, 255)

os.makedirs("assets", exist_ok=True)

project = {
    "name": "Demo Platformer",
    "viewportWidth": 800,
    "viewportHeight": 480,
    "firstLayout": "Layout 1",
    "properties": {"backgroundColor": [0.13, 0.15, 0.19, 1.0]},
}

objectTypes = {
    "Player": {
        "name": "Player", "plugin-id": "sprite", "isFamily": False,
        "instanceVariables": [{"name": "lives", "type": "number", "initialValue": 3}],
        "behaviorTypes": [
            {"name": "Platform", "properties": {"max-speed": 260, "acceleration": 1400, "deceleration": 1400,
                                                  "jump-strength": 640, "gravity": 1500, "maximum-fall-speed": 900,
                                                  "default-controls": 1}},
            {"name": "ScrollTo", "properties": {}},
        ],
        "animations": {"items": [{"name": "Default", "speed": 0, "isLooping": False, "frames": [{}]}]},
    },
    "Ground": {
        "name": "Ground", "plugin-id": "tiledbg", "isFamily": False,
        "behaviorTypes": [{"name": "Solid", "properties": {}}],
        "animations": {"items": [{"name": "Default", "speed": 0, "isLooping": False, "frames": [{}]}]},
    },
    "Coin": {
        "name": "Coin", "plugin-id": "sprite", "isFamily": False,
        "instanceVariables": [{"name": "value", "type": "number", "initialValue": 10}],
        "behaviorTypes": [{"name": "Sine", "properties": {"movement": "vertical", "period": 1.4, "magnitude": 8}}],
        "animations": {"items": [{"name": "Default", "speed": 0, "isLooping": False, "frames": [{}]}]},
    },
    "Enemy": {
        "name": "Enemy", "plugin-id": "sprite", "isFamily": False,
        "behaviorTypes": [{"name": "Bullet", "properties": {"speed": 120, "bounce-off-solids": 1}},
                          {"name": "DestroyOutsideLayout", "properties": {}}],
        "animations": {"items": [{"name": "Default", "speed": 0, "isLooping": False, "frames": [{}]}]},
    },
    "Hud": {
        "name": "Hud", "plugin-id": "text", "isFamily": False,
        "animations": {"items": []},
    },
    "Keyboard": {"name": "Keyboard", "plugin-id": "keyboard", "isFamily": False},
    "Mouse": {"name": "Mouse", "plugin-id": "mouse", "isFamily": False},
    "Background": {
        "name": "Background", "plugin-id": "tiledbg", "isFamily": False,
        "animations": {"items": [{"name": "Default", "speed": 0, "isLooping": False, "frames": [{}]}]},
    },
}

def inst(t, x, y, w, h, uid, ox=0.5, oy=0.5, vars=None, props=None):
    return {
        "type": t, "uid": uid,
        "world": {"x": x, "y": y, "width": w, "height": h, "angle": 0.0,
                  "originX": ox, "originY": oy, "color": [1, 1, 1, 1]},
        "instanceVariables": vars or {},
        "properties": props or {},
        "behaviors": {},
    }

ground_instances = []
uid = 100
for i in range(8):
    ground_instances.append(inst("Ground", 96 + i * 128, 400, 128, 32, uid, ox=0.0, oy=0.0))
    uid += 1
ground_instances.append(inst("Ground", 700, 260, 192, 32, uid, ox=0.0, oy=0.0))
uid += 1

coins = [inst("Coin", 260 + i * 110, 330, 32, 32, uid, vars={"value": 10}) for i, uid in enumerate(range(200, 205))]

layout = {
    "name": "Layout 1",
    "width": 1280, "height": 640,
    "backgroundColor": [0.13, 0.15, 0.19, 1.0],
    "eventSheet": "Feuille 1",
    "layers": [
        {"name": "Fond", "parallaxX": 40, "parallaxY": 40, "visible": True, "opacity": 100,
         "instances": [inst("Background", 640, 320, 1280, 640, 50)]},
        {"name": "Principal", "parallaxX": 100, "parallaxY": 100, "visible": True, "opacity": 100,
         "instances": ground_instances + coins + [
             inst("Player", 120, 340, 64, 64, 300, vars={"lives": 3}),
             inst("Enemy", 900, 200, 32, 32, 301),
             inst("Enemy", 1000, 150, 32, 32, 302),
         ]},
        {"name": "UI", "parallaxX": 0, "parallaxY": 0, "visible": True, "opacity": 100,
         "instances": [inst("Hud", 16, 12, 400, 30, 400, ox=0.0, oy=0.0,
                            props={"text": "Score: 0", "font-size": 22, "color": "#FFFFFFFF"})]},
    ],
}

events = [
    {"eventType": "variable", "name": "Score", "type": "number", "initialValue": "0", "isConstant": False},
    {
        "eventType": "block", "disabled": False, "sid": 1001,
        "conditions": [{"id": "on-start-of-layout", "objectClass": "System", "parameters": {}, "sid": 1}],
        "actions": [{"id": "set-text", "objectClass": "Hud", "parameters": {"text": "\"Score: \" & Score"}, "sid": 2}],
        "children": [],
    },
    {
        "eventType": "block", "disabled": False, "sid": 1002,
        "conditions": [
            {"id": "on-collision-with-another-object", "objectClass": "Player",
             "parameters": {"object": "Coin"}, "sid": 3},
        ],
        "actions": [
            {"id": "add-to-eventvar", "objectClass": "System", "parameters": {"variable": "Score", "value": "Coin.value"}, "sid": 4},
            {"id": "set-text", "objectClass": "Hud", "parameters": {"text": "\"Score: \" & Score"}, "sid": 5},
            {"id": "destroy", "objectClass": "Coin", "parameters": {}, "sid": 6},
        ],
        "children": [],
    },
    {
        "eventType": "block", "disabled": False, "sid": 1003,
        "conditions": [{"id": "on-collision-with-another-object", "objectClass": "Player",
                        "parameters": {"object": "Enemy"}, "sid": 7}],
        "actions": [{"id": "restart-layout", "objectClass": "System", "parameters": {}, "sid": 8}],
        "children": [],
    },
]

sheet = {"name": "Feuille 1", "events": events}

images = {
    "player.png": png(64, 64, player_px),
    "enemy.png": png(32, 32, enemy_px),
    "coin.png": png(32, 32, coin_px),
    "ground.png": png(32, 32, tile_px),
    "background.png": png(32, 32, bg_px),
}

out = "assets/sample.c3p"
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
    z.writestr("project.c3proj", json.dumps(project))
    for name, doc in objectTypes.items():
        z.writestr(f"objectTypes/{name}.json", json.dumps(doc))
    z.writestr("layouts/Layout 1.json", json.dumps(layout))
    z.writestr("eventSheets/Feuille 1.json", json.dumps(sheet))
    for src, data in images.items():
        z.writestr(f"images/{src}", data)

print("écrit :", out, os.path.getsize(out), "octets")
