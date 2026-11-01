#!/usr/bin/env python3
"""Writes the content half of Create for Horizon (`content/`): the blocks and
items as Horizon content-mod declarations, their block states, models and
textures, and Create's recipes, loot tables and tags.

Recipes, loot tables and tags follow Create's generated data (MIT). Create's
textures and models are All Rights Reserved, so none are used: the textures
here are drawn by this script, and the models are simple boxes of its own.

Run: python3 tools/build_content.py   (needs Pillow)
"""
import json
import random
import shutil
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent / "content"
NS = "create"


def write(path, data):
    p = ROOT / path
    p.parent.mkdir(parents=True, exist_ok=True)
    if isinstance(data, (dict, list)):
        p.write_text(json.dumps(data, indent=2) + "\n")
    else:
        p.write_text(data)


# ---- blocks -----------------------------------------------------------------

AXIS = {"type": "enum", "values": ["y", "x", "z"]}
FACING = {"type": "enum", "values": ["up", "down", "north", "south", "west", "east"]}
METAL = {"hardness": 1.5, "explosion_resistance": 6.0, "sounds": "minecraft:netherite_block", "light": 0, "map_color": 6}
WOOD = {"hardness": 1.5, "explosion_resistance": 6.0, "sounds": "minecraft:oak_planks", "light": 0, "map_color": 13}
STONE = {"hardness": 2.0, "explosion_resistance": 6.0, "sounds": "minecraft:stone", "light": 0, "map_color": 11}

FULL = [[0, 0, 0, 1, 1, 1]]
SHAFT_JACK = [
    [0.375, 0, 0.375, 0.625, 1, 0.625],
    [0, 0.375, 0.375, 1, 0.625, 0.625],
    [0.375, 0.375, 0, 0.625, 0.625, 1],
]
COG = [[0.0625, 0.0625, 0.0625, 0.9375, 0.9375, 0.9375]]
CRANK = [[0.125, 0.125, 0.125, 0.875, 0.875, 0.875]]


def shapes(boxes):
    # The conduit template's own property: the same boxes either way.
    return {"property": "waterlogged", "values": {"false": boxes, "true": boxes}}


# Every kinetic block copies the conduit: a block entity, no full face, no menu.
KINETIC = {
    "shaft": (METAL, AXIS, SHAFT_JACK),
    "cogwheel": (WOOD, AXIS, COG),
    "large_cogwheel": (WOOD, AXIS, FULL),
    "gearbox": (WOOD, AXIS, FULL),
    "creative_motor": (METAL, FACING, FULL),
    "hand_crank": (WOOD, FACING, CRANK),
    "speedometer": (WOOD, AXIS, FULL),
    "stressometer": (WOOD, AXIS, FULL),
    "mechanical_press": (WOOD, AXIS, FULL),
    "millstone": (STONE, None, FULL),
}

NAMES = {
    "shaft": ("Shaft", "Arbre de transmission"),
    "cogwheel": ("Cogwheel", "Roue dentée"),
    "large_cogwheel": ("Large Cogwheel", "Grande roue dentée"),
    "gearbox": ("Gearbox", "Boîte de vitesses"),
    "creative_motor": ("Creative Motor", "Moteur créatif"),
    "hand_crank": ("Hand Crank", "Manivelle"),
    "speedometer": ("Speedometer", "Compteur de vitesse"),
    "stressometer": ("Stressometer", "Compteur de stress"),
    "mechanical_press": ("Mechanical Press", "Presse mécanique"),
    "millstone": ("Millstone", "Meule"),
    "andesite_casing": ("Andesite Casing", "Boîtier en andésite"),
    "andesite_alloy": ("Andesite Alloy", "Alliage d'andésite"),
}


def blocks():
    for name, (props, prop, boxes) in KINETIC.items():
        decl = {"template": "minecraft:conduit", "properties": props, "shapes": shapes(boxes), "item": True}
        if prop is not None:
            key = "facing" if prop is FACING else "axis"
            decl["state_properties"] = {key: prop}
        write(f"data/{NS}/horizon/block/{name}.json", decl)
    write(
        f"data/{NS}/horizon/block/andesite_casing.json",
        {"template": "minecraft:oak_planks", "properties": WOOD, "item": True},
    )
    write(f"data/{NS}/horizon/item/andesite_alloy.json", {"template": "minecraft:iron_ingot"})
    write(
        f"data/{NS}/horizon/creative_tab/base.json",
        {
            "title": "itemGroup.create.base",
            "icon": f"{NS}:cogwheel",
            "items": [f"{NS}:{n}" for n in [
                "andesite_alloy", "andesite_casing", "shaft", "cogwheel", "large_cogwheel", "gearbox",
                "hand_crank", "creative_motor", "speedometer", "stressometer", "mechanical_press", "millstone",
            ]],
        },
    )


# ---- models -----------------------------------------------------------------

def tex(n):
    return f"{NS}:block/{n}"


def el(fr, to, faces, rotation=None):
    e = {"from": fr, "to": to, "faces": {}}
    for f, t in faces.items():
        e["faces"][f] = {"texture": t}
    if rotation:
        e["rotation"] = rotation
    return e


SIX = ["down", "up", "north", "south", "west", "east"]


def box(fr, to, side, end=None):
    end = end or side
    return el(fr, to, {f: (end if f in ("up", "down") else side) for f in SIX})


def model(name, elements, textures, parent=None):
    m = {"textures": textures, "elements": elements}
    if parent:
        m = {"parent": parent, "textures": textures}
    write(f"assets/{NS}/models/block/{name}.json", m)


def models():
    # Turning parts are drawn by the block entity; the block's own model holds
    # only what stands still (and the particle).
    model("shaft", [], {"particle": tex("axis")})
    model("cogwheel", [], {"particle": tex("cogwheel")})
    model("large_cogwheel", [], {"particle": tex("large_cogwheel")})
    model("gearbox", [box([0, 0, 0], [16, 16, 16], "#side", "#end")],
          {"side": tex("gearbox"), "end": tex("andesite_casing"), "particle": tex("andesite_casing")})
    model("creative_motor", [
        box([2, 0, 2], [14, 3, 14], "#casing"),
        box([3, 3, 3], [13, 10, 13], "#body", "#casing"),
        box([5, 10, 5], [11, 12, 11], "#casing"),
    ], {"casing": tex("andesite_casing"), "body": tex("creative_motor"), "particle": tex("creative_motor")})
    model("hand_crank", [box([4, 0, 4], [12, 2, 12], "#base")],
          {"base": tex("andesite_casing"), "particle": tex("hand_crank")})
    for g in ("speedometer", "stressometer"):
        model(g, [box([0, 0, 0], [16, 16, 16], "#face", "#end")],
              {"face": tex(f"{g}_face"), "end": tex("andesite_casing"), "particle": tex("andesite_casing")})
    model("mechanical_press", [
        box([0, 2, 0], [16, 16, 16], "#side", "#casing"),
        box([3, 0, 3], [13, 2, 13], "#head"),
    ], {"side": tex("mechanical_press"), "casing": tex("andesite_casing"), "head": tex("axis_top"),
        "particle": tex("mechanical_press")})
    model("millstone", [box([0, 0, 0], [16, 16, 16], "#side", "#top")],
          {"side": tex("millstone"), "top": tex("millstone_top"), "particle": tex("millstone")})
    model("andesite_casing", [], {"all": tex("andesite_casing")}, parent="minecraft:block/cube_all")

    # Item models: the turning parts, standing still.
    def item_model(name, elements, textures):
        write(f"assets/{NS}/models/item/{name}.json",
              {"parent": "minecraft:block/block", "textures": textures, "elements": elements})
        write(f"assets/{NS}/items/{name}.json",
              {"model": {"type": "minecraft:model", "model": f"{NS}:item/{name}"}})

    shaft_el = box([6, 0, 6], [10, 16, 10], "#axis", "#axis_top")
    shaft_t = {"axis": tex("axis"), "axis_top": tex("axis_top"), "particle": tex("axis")}
    item_model("shaft", [shaft_el], shaft_t)
    cog = [shaft_el, box([1, 6, 1], [15, 10, 15], "#cog_side", "#cog")]
    item_model("cogwheel", cog, {**shaft_t, "cog": tex("cogwheel"), "cog_side": tex("cogwheel_side")})
    large = [shaft_el, box([-4, 6, -4], [20, 10, 20], "#cog_side", "#cog")]
    item_model("large_cogwheel", large, {**shaft_t, "cog": tex("large_cogwheel"), "cog_side": tex("cogwheel_side")})
    crank = [box([4, 0, 4], [12, 2, 12], "#base"), box([7, 2, 7], [9, 6, 9], "#handle"),
             box([7, 5, 2], [9, 7, 14], "#handle"), box([7, 5, 12], [9, 12, 14], "#handle")]
    item_model("hand_crank", crank, {"base": tex("andesite_casing"), "handle": tex("hand_crank"),
                                     "particle": tex("hand_crank")})
    for n in ("gearbox", "creative_motor", "speedometer", "stressometer", "mechanical_press", "millstone",
              "andesite_casing"):
        write(f"assets/{NS}/items/{n}.json",
              {"model": {"type": "minecraft:model", "model": f"{NS}:block/{n}"}})
    write(f"assets/{NS}/models/item/andesite_alloy.json",
          {"parent": "minecraft:item/generated", "textures": {"layer0": f"{NS}:item/andesite_alloy"}})
    write(f"assets/{NS}/items/andesite_alloy.json",
          {"model": {"type": "minecraft:model", "model": f"{NS}:item/andesite_alloy"}})


def blockstates():
    def one(name):
        write(f"assets/{NS}/blockstates/{name}.json", {"variants": {"": {"model": f"{NS}:block/{name}"}}})

    def pillar(name):
        rot = {"y": {}, "x": {"x": 90, "y": 90}, "z": {"x": 90}}
        write(f"assets/{NS}/blockstates/{name}.json", {"variants": {
            f"axis={a}": {"model": f"{NS}:block/{name}", **r} for a, r in rot.items()}})

    def facing(name):
        rot = {"up": {}, "down": {"x": 180}, "north": {"x": 90}, "south": {"x": 90, "y": 180},
               "west": {"x": 90, "y": 270}, "east": {"x": 90, "y": 90}}
        write(f"assets/{NS}/blockstates/{name}.json", {"variants": {
            f"facing={f}": {"model": f"{NS}:block/{name}", **r} for f, r in rot.items()}})

    for n in ("shaft", "cogwheel", "large_cogwheel", "millstone", "andesite_casing"):
        one(n)
    for n in ("gearbox", "speedometer", "stressometer", "mechanical_press"):
        pillar(n)
    for n in ("creative_motor", "hand_crank"):
        facing(n)


# ---- textures (drawn here) ----------------------------------------------------

def png(path, img):
    p = ROOT / f"assets/{NS}/textures/{path}.png"
    p.parent.mkdir(parents=True, exist_ok=True)
    img.save(p)


def noise(base, spread, seed):
    rnd = random.Random(seed)
    img = Image.new("RGBA", (16, 16))
    for x in range(16):
        for y in range(16):
            d = rnd.randint(-spread, spread)
            img.putpixel((x, y), tuple(max(0, min(255, c + d)) for c in base) + (255,))
    return img


def frame(img, color, width=1):
    d = ImageDraw.Draw(img)
    for i in range(width):
        d.rectangle([i, i, 15 - i, 15 - i], outline=color)
    return img


def textures():
    andesite = (148, 151, 141)
    casing = frame(noise(andesite, 8, 1), (96, 74, 52))
    ImageDraw.Draw(casing).rectangle([1, 1, 14, 14], outline=(122, 98, 70))
    png("block/andesite_casing", casing)

    axis = noise((170, 172, 165), 6, 2)
    d = ImageDraw.Draw(axis)
    for y in range(0, 16, 4):
        d.line([0, y, 15, y], fill=(120, 122, 116))
    png("block/axis", axis)
    top = noise((170, 172, 165), 5, 3)
    ImageDraw.Draw(top).rectangle([0, 0, 15, 15], outline=(110, 112, 106))
    ImageDraw.Draw(top).rectangle([6, 6, 9, 9], fill=(90, 92, 86))
    png("block/axis_top", top)

    plank = (164, 126, 78)
    cog = noise(plank, 10, 4)
    d = ImageDraw.Draw(cog)
    d.ellipse([1, 1, 14, 14], outline=(110, 82, 48))
    d.ellipse([5, 5, 10, 10], fill=(150, 152, 145), outline=(90, 92, 86))
    png("block/cogwheel", cog)
    side = noise(plank, 10, 5)
    ImageDraw.Draw(side).line([0, 8, 15, 8], fill=(120, 90, 54))
    png("block/cogwheel_side", side)
    large = noise((150, 112, 66), 10, 6)
    d = ImageDraw.Draw(large)
    for r in (1, 4):
        d.ellipse([r, r, 15 - r, 15 - r], outline=(100, 74, 42))
    d.ellipse([6, 6, 9, 9], fill=(150, 152, 145))
    png("block/large_cogwheel", large)

    gear = casing.copy()
    d = ImageDraw.Draw(gear)
    d.ellipse([4, 4, 11, 11], fill=(60, 50, 40))
    d.ellipse([6, 6, 9, 9], fill=(150, 152, 145))
    png("block/gearbox", gear)

    motor = frame(noise((118, 72, 168), 10, 7), (70, 40, 110))
    ImageDraw.Draw(motor).line([2, 8, 13, 8], fill=(220, 190, 255))
    png("block/creative_motor", motor)

    png("block/hand_crank", noise(plank, 8, 8))

    for g, ink in (("speedometer", (40, 200, 60)), ("stressometer", (220, 60, 40))):
        face = casing.copy()
        d = ImageDraw.Draw(face)
        d.ellipse([2, 2, 13, 13], fill=(232, 228, 214), outline=(70, 60, 50))
        d.arc([3, 3, 12, 12], 135, 405, fill=ink)
        d.point((8, 8), fill=(40, 40, 40))
        png(f"block/{g}_face", face)
    needle = Image.new("RGBA", (16, 16), (200, 30, 30, 255))
    png("block/gauge_needle", needle)

    press = casing.copy()
    ImageDraw.Draw(press).rectangle([3, 3, 12, 12], fill=(108, 110, 104))
    png("block/mechanical_press", press)
    mill = frame(noise((130, 130, 130), 12, 9), (90, 90, 90))
    png("block/millstone", mill)
    mtop = noise((140, 140, 140), 12, 10)
    ImageDraw.Draw(mtop).ellipse([2, 2, 13, 13], outline=(80, 80, 80))
    png("block/millstone_top", mtop)

    alloy = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
    d = ImageDraw.Draw(alloy)
    d.polygon([(2, 9), (8, 5), (14, 8), (8, 12)], fill=(160, 163, 152), outline=(92, 95, 86))
    d.line([(4, 9), (8, 7), (12, 8)], fill=(200, 202, 194))
    png("item/andesite_alloy", alloy)


# ---- data (Create's, MIT) ---------------------------------------------------------

def item(i):
    return {"item": i}


def shaped(name, pattern, key, result, count=1):
    write(f"data/{NS}/recipe/crafting/{name}.json", {
        "type": "minecraft:crafting_shaped", "category": "misc", "key": key, "pattern": pattern,
        "result": {"count": count, "id": result}})


def shapeless(name, ingredients, result, count=1):
    write(f"data/{NS}/recipe/crafting/{name}.json", {
        "type": "minecraft:crafting_shapeless", "category": "misc", "ingredients": ingredients,
        "result": {"count": count, "id": result}})


def data():
    c = lambda n: f"{NS}:{n}"
    planks = {"tag": "minecraft:planks"}
    shaped("materials/andesite_alloy", ["BA", "AB"],
           {"A": item("minecraft:andesite"), "B": {"tag": "c:nuggets/iron"}}, c("andesite_alloy"))
    shaped("kinetics/shaft", ["A", "A"], {"A": item(c("andesite_alloy"))}, c("shaft"), 8)
    shapeless("kinetics/cogwheel", [item(c("shaft")), planks], c("cogwheel"))
    shapeless("kinetics/large_cogwheel", [item(c("shaft")), planks, planks], c("large_cogwheel"))
    shapeless("kinetics/large_cogwheel_from_little", [item(c("cogwheel")), planks], c("large_cogwheel"))
    shaped("kinetics/gearbox", [" C ", "CBC", " C "],
           {"B": item(c("andesite_casing")), "C": item(c("cogwheel"))}, c("gearbox"))
    shaped("kinetics/hand_crank", ["CCC", "  A"], {"A": item(c("andesite_alloy")), "C": planks}, c("hand_crank"))
    shaped("kinetics/speedometer", ["C", "A"],
           {"A": item(c("andesite_casing")), "C": item("minecraft:compass")}, c("speedometer"))
    shapeless("kinetics/stressometer_from_conversion", [item(c("speedometer"))], c("stressometer"))
    shapeless("kinetics/speedometer_from_conversion", [item(c("stressometer"))], c("speedometer"))
    shaped("kinetics/mechanical_press", ["S", "C", "I"],
           {"C": item(c("andesite_casing")), "I": {"tag": "c:storage_blocks/iron"}, "S": item(c("shaft"))},
           c("mechanical_press"))
    shaped("kinetics/millstone", ["C", "S", "I"],
           {"C": item(c("cogwheel")), "I": {"tag": "c:stones"}, "S": item(c("andesite_casing"))}, c("millstone"))

    for b in list(KINETIC) + ["andesite_casing"]:
        write(f"data/{NS}/loot_table/blocks/{b}.json", {
            "type": "minecraft:block",
            "pools": [{"bonus_rolls": 0.0, "conditions": [{"condition": "minecraft:survives_explosion"}],
                       "entries": [{"type": "minecraft:item", "name": c(b)}], "rolls": 1.0}],
            "random_sequence": f"{NS}:blocks/{b}"})

    pick = list(KINETIC) + ["andesite_casing"]
    axe = ["cogwheel", "large_cogwheel", "gearbox", "hand_crank", "mechanical_press", "speedometer",
           "stressometer", "andesite_casing"]
    write("data/minecraft/tags/block/mineable/pickaxe.json", {"values": [c(b) for b in pick]})
    write("data/minecraft/tags/block/mineable/axe.json", {"values": [c(b) for b in axe]})
    write(f"data/{NS}/tags/block/casing.json", {"values": [c("andesite_casing")]})
    # The convention tags Create's recipes use, as NeoForge fills them.
    write("data/c/tags/item/nuggets/iron.json", {"values": ["minecraft:iron_nugget"]})
    write("data/c/tags/item/storage_blocks/iron.json", {"values": ["minecraft:iron_block"]})
    write("data/c/tags/item/stones.json", {"values": [
        "minecraft:stone", "minecraft:andesite", "minecraft:diorite", "minecraft:granite",
        "minecraft:tuff", "minecraft:deepslate"]})
    logs = [f"minecraft:stripped_{w}_{k}" for w in (
        "oak", "spruce", "birch", "jungle", "acacia", "dark_oak", "mangrove", "cherry", "pale_oak")
        for k in ("log", "wood")] + [f"minecraft:stripped_{w}_{k}" for w in ("crimson", "warped")
                                     for k in ("stem", "hyphae")] + ["minecraft:stripped_bamboo_block"]
    write("data/c/tags/block/stripped_logs.json", {"values": logs})
    write("data/c/tags/item/stripped_logs.json", {"values": logs})


def lang():
    en = {f"block.{NS}.{k}": v[0] for k, v in NAMES.items() if k != "andesite_alloy"}
    fr = {f"block.{NS}.{k}": v[1] for k, v in NAMES.items() if k != "andesite_alloy"}
    en[f"item.{NS}.andesite_alloy"], fr[f"item.{NS}.andesite_alloy"] = NAMES["andesite_alloy"]
    en["itemGroup.create.base"] = "Create"
    fr["itemGroup.create.base"] = "Create"
    write(f"assets/{NS}/lang/en_us.json", en)
    write(f"assets/{NS}/lang/fr_fr.json", fr)


def manifest():
    write("horizon-mod.toml", "\n".join([
        f'id = "{NS}"',
        'name = "Create"',
        'version = "0.1.0"',
        'description = "Create\'s blocks for Horizon: shafts, cogwheels, gearboxes, motors, cranks, gauges. '
        'Needs the create_horizon native mod for their kinetics."',
        'authors = ["Create for Horizon contributors"]',
        'api = "0.2"',
        'side = "both"',
        'capabilities = ["registries"]',
        "",
    ]))


if __name__ == "__main__":
    if ROOT.exists():
        shutil.rmtree(ROOT)
    manifest()
    blocks()
    models()
    blockstates()
    textures()
    data()
    lang()
    print("wrote", sum(1 for _ in ROOT.rglob("*") if _.is_file()), "files to", ROOT)
