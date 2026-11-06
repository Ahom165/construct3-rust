#!/usr/bin/env python3
"""Génère demo/demo-platformer.c3p — un petit jeu de plateforme original
au format natif C3R, avec sprites PNG et son WAV générés par le script."""
import json, struct, wave, zipfile, io, math, os, random

OUT = os.path.join(os.path.dirname(__file__), "..", "demo", "demo-platformer.c3p")

def png(w, h, pix):
    def chunk(typ, data):
        c = struct.pack(">I", len(data)) + typ + data
        return c + struct.pack(">I", zlib.crc32(typ + data) & 0xffffffff)
    raw = b""
    for y in range(h):
        raw += b"\x00"
        for x in range(w):
            raw += bytes(pix(x, y))
    ihdr = struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr)
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))

import zlib

def player_sprite():  # 24x36 petit héros bleu
    w, h = 24, 36
    def pix(x, y):
        border = x in (0, w-1) or y in (0, h-1)
        head = y < 12
        eye = (5 <= x <= 7 or 16 <= x <= 18) and 4 <= y <= 6
        legs = y >= 30 and (x < 8 or x > 15) and y % 6 < 4
        if eye: return (20, 20, 30, 255)
        if border: return (30, 60, 140, 255)
        if head: return (255, 205, 160, 255)
        if legs: return (40, 45, 90, 255)
        return (60, 110, 220, 255)
    return png(w, h, pix)

def ground_tile():  # 32x32 herbe + terre
    w, h = 32, 32
    def pix(x, y):
        if y < 8:
            return (20 + (x*7 % 30), 160 + (x*13 % 40), 60, 255)
        if y == 8: return (120, 90, 40, 255)
        return (150 + ((x*3+y*7) % 25), 110 + ((x+y*3) % 20), 60, 255)
    return png(w, h, pix)

def coin():  # 20x20 pièce
    w, h = 20, 20
    def pix(x, y):
        dx, dy = x-9.5, y-9.5
        d = math.sqrt(dx*dx+dy*dy)
        if d > 9: return (0, 0, 0, 0)
        if d > 7: return (180, 130, 20, 255)
        if abs(dx) < 2: return (255, 230, 120, 255)
        return (250, 200, 40, 255)
    return png(w, h, pix)

def enemy():  # 28x28 blob rouge
    w, h = 28, 28
    def pix(x, y):
        dx, dy = x-13.5, y-14
        d = math.sqrt(dx*dx + (dy*1.15)**2)
        if d > 13: return (0, 0, 0, 0)
        eye = (7 <= x <= 10 or 17 <= x <= 20) and 9 <= y <= 13
        if eye: return (255, 255, 255, 255)
        if 8 <= x <= 9 or 18 <= x <= 19:
            if 10 <= y <= 12: return (10, 10, 10, 255)
        if d > 11: return (120, 20, 20, 255)
        return (220, 50, 50, 255)
    return png(w, h, pix)

def coin_wav():
    sr = 22050
    dur = 0.16
    buf = io.BytesIO()
    w = wave.open(buf, "wb")
    w.setnchannels(1); w.setsampwidth(2); w.setframerate(sr)
    frames = bytearray()
    for i in range(int(sr*dur)):
        t = i/sr
        env = 1.0 - t/dur
        v = math.sin(2*math.pi*987*t) * 0.5 + math.sin(2*math.pi*1318*t)*0.3
        frames += struct.pack("<h", int(v*env*22000))
    w.writeframes(bytes(frames)); w.close()
    return buf.getvalue()

def data_uri(mime, b):
    import base64
    return "data:%s;base64,%s" % (mime, base64.b64encode(b).decode())

assets = [
    {"id": "tx1", "name": "player.png", "kind": "image", "mime": "image/png", "data": data_uri("image/png", player_sprite()), "width": 24, "height": 36},
    {"id": "tx2", "name": "ground.png", "kind": "image", "mime": "image/png", "data": data_uri("image/png", ground_tile()), "width": 32, "height": 32},
    {"id": "tx3", "name": "coin.png", "kind": "image", "mime": "image/png", "data": data_uri("image/png", coin()), "width": 20, "height": 20},
    {"id": "tx4", "name": "enemy.png", "kind": "image", "mime": "image/png", "data": data_uri("image/png", enemy()), "width": 28, "height": 28},
    {"id": "a1", "name": "coin.wav", "kind": "audio", "mime": "audio/wav", "data": data_uri("audio/wav", coin_wav()), "width": 0, "height": 0},
]

types = [
    {"uid": 1, "name": "Player", "plugin": "sprite", "is_global": False,
     "behaviors": [{"id": "platform", "name": "Platform",
                    "properties": {"max-speed": 260, "gravity": 1500, "jump-strength": 640,
                                   "acceleration": 1400, "deceleration": 1400}}],
     "instance_vars": [], "animations": [{"name": "Default", "speed": 5, "looping": True,
                                          "frames": [{"texture": "tx1", "duration": 1}]}],
     "properties": {}, "texture": ""},
    {"uid": 2, "name": "Ground", "plugin": "tiledbg", "is_global": False,
     "behaviors": [{"id": "solid", "name": "Solid", "properties": {}}],
     "instance_vars": [], "animations": [], "properties": {"texture": "tx2"}, "texture": "tx2"},
    {"uid": 3, "name": "Coin", "plugin": "sprite", "is_global": False,
     "behaviors": [{"id": "sine", "name": "Sine", "properties": {"movement": "vertical", "period": 1.2, "range": 8}}],
     "instance_vars": [], "animations": [{"name": "Default", "speed": 5, "looping": True,
                                          "frames": [{"texture": "tx3", "duration": 1}]}],
     "properties": {}, "texture": ""},
    {"uid": 4, "name": "Enemy", "plugin": "sprite", "is_global": False,
     "behaviors": [{"id": "bullet", "name": "Bullet", "properties": {"speed": 110}},
                   {"id": "destroyoutside", "name": "DestroyOutside", "properties": {"margin": 100}}],
     "instance_vars": [], "animations": [{"name": "Default", "speed": 5, "looping": True,
                                          "frames": [{"texture": "tx4", "duration": 1}]}],
     "properties": {}, "texture": ""},
    {"uid": 5, "name": "ScoreText", "plugin": "text", "is_global": False,
     "behaviors": [], "instance_vars": [], "animations": [],
     "properties": {"text": "Score : 0", "size": 26, "color": "#ffffff"}, "texture": ""},
    {"uid": 6, "name": "Keyboard", "plugin": "keyboard", "is_global": True,
     "behaviors": [], "instance_vars": [], "animations": [], "properties": {}, "texture": ""},
    {"uid": 7, "name": "Audio", "plugin": "audio", "is_global": True,
     "behaviors": [], "instance_vars": [], "animations": [], "properties": {}, "texture": ""},
]

def inst(uid, obj, x, y, w, h, props=None):
    return {"uid": uid, "object": obj, "x": x, "y": y, "width": w, "height": h,
            "angle": 0, "z": 0, "opacity": 100, "vars": {}, "properties": props or {}}

instances = [
    inst(1, "ScoreText", 24, 18, 240, 34, {"text": "Score : 0", "size": 26, "color": "#ffffff"}),
    inst(2, "Ground", 0, 656, 1280, 64),
    inst(3, "Ground", 384, 480, 384, 32),
    inst(4, "Ground", 768, 352, 256, 32),
    inst(5, "Player", 90, 560, 24, 36),
]
x = 260
cid = 10
for i in range(8):
    instances.append(inst(cid, "Coin", x, 540, 20, 20)); cid += 1; x += 90
instances.append(inst(cid, "Coin", 480, 440, 20, 20)); cid += 1
instances.append(inst(cid, "Coin", 560, 440, 20, 20)); cid += 1
instances.append(inst(cid, "Coin", 850, 310, 20, 20)); cid += 1
instances.append(inst(cid, "Coin", 920, 310, 20, 20)); cid += 1
for ex in (1500, 1900):
    instances.append(inst(cid, "Enemy", ex, 610, 28, 28, {})); cid += 1
# sol étendu pour la zone des ennemis
instances.append(inst(cid, "Ground", 1280, 656, 1280, 64)); cid += 1

layout = {
    "uid": 10, "name": "Game", "width": 2560, "height": 720, "background": "#7ec8f7",
    "layers": [{"name": "Layer 0", "visible": True, "opacity": 100,
                "parallax_x": 100, "parallax_y": 100, "instances": instances}],
    "event_sheet": "Game events",
}

E = lambda c, a: {"disabled": False, "is_group": False, "group_name": "", "conditions": c, "actions": a, "children": []}
C = lambda cid, obj, *p: {"id": cid, "object": obj, "params": list(p)}
A = lambda aid, obj, *p: {"id": aid, "object": obj, "params": list(p)}

events = [
    E([C("on_start_of_layout", "system")],
      [A("set_text", "ScoreText", "Score : 0")]),
    E([C("on_collision_with", "Player", "Coin")],
      [A("destroy", "Coin"),
       A("set_global_var", "system", "score", "add", 1),
       A("set_text", "ScoreText", "Score : {score}"),
       A("play_sound", "system", "coin.wav", "0", "0.8")]),
    E([C("on_collision_with", "Player", "Enemy")],
      [A("set_global_var", "system", "score", "set", 0),
       A("restart_layout", "system")]),
    E([C("on_object_clicked", "Enemy")],
      [A("destroy", "Enemy"),
       A("set_global_var", "system", "score", "add", 5),
       A("set_text", "ScoreText", "Score : {score}")]),
    E([C("key_pressed", "Keyboard", "r")],
      [A("restart_layout", "system")]),
]

project = {
    "format": "c3r-1",
    "name": "Platformer Demo",
    "author": "C3R",
    "description": "Jeu de plateforme de démonstration pour C3R (original).",
    "version": "1.0",
    "width": 1280, "height": 720,
    "first_layout": "Game",
    "object_types": types,
    "layouts": [layout],
    "event_sheets": [{"uid": 11, "name": "Game events", "events": events}],
    "global_vars": [{"name": "score", "type": "number", "initial": 0}],
    "assets": assets,
}

os.makedirs(os.path.dirname(OUT), exist_ok=True)
with zipfile.ZipFile(OUT, "w", zipfile.ZIP_DEFLATED) as z:
    z.writestr("project.json", json.dumps(project, indent=1, ensure_ascii=False))
print("OK:", os.path.abspath(OUT), os.path.getsize(OUT), "octets")
