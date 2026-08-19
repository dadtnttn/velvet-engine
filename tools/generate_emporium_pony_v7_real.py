import json
import os
import shutil
import time
import urllib.request

COMFY_URL = "http://127.0.0.1:8188"
DEST_DIR = r"demos\summoner-emporium\data\assets"

prompts = [
    {
        "name": "store",
        "sub": "backgrounds/png",
        "prompt": "score_9, score_8_up, score_7_up, score_6_up, score_5_up, score_4_up, rating_safe, photorealistic, photograph, 24mm wide lens, interior photography, modern luxury store interior, sleek glass showcases, glowing ambient neon accents, polished dark marble floors, premium boutique display, sharp focus, 8k uhd photo",
        "lora_char": None,
        "width": 1216,
        "height": 832
    },
    {
        "name": "altar",
        "sub": "backgrounds/png",
        "prompt": "score_9, score_8_up, score_7_up, score_6_up, score_5_up, score_4_up, rating_safe, photorealistic, photograph, 24mm wide lens, high-tech summoning chamber inside a penthouse suite, glowing purple holographic summoning ring on dark glass floor, neon blue ambient highlights, sharp focus, 8k uhd photo",
        "lora_char": None,
        "width": 1216,
        "height": 832
    },
    {
        "name": "vip",
        "sub": "backgrounds/png",
        "prompt": "score_9, score_8_up, score_7_up, score_6_up, score_5_up, score_4_up, rating_safe, photorealistic, photograph, 24mm wide lens, modern luxury VIP lounge penthouse suite, plush leather sofa, warm romantic mood lighting, panoramic city night view through floor-to-ceiling glass windows, sharp focus, 8k uhd photo",
        "lora_char": None,
        "width": 1216,
        "height": 832
    },
    {
        "name": "sam_neutral",
        "sub": "tomboy",
        "prompt": "score_9, score_8_up, score_7_up, score_6_up, score_5_up, score_4_up, rating_safe, 1girl, ashleigh, photorealistic, photograph, 85mm lens, f/1.8, softbox studio lighting, highly detailed skin texture, sharp focus, portrait, photo of an attractive woman, stylish modern leather jacket, confident expression",
        "lora_char": "MisterMR_Ashleigh_Phoenix_Pony.safetensors",
        "width": 832,
        "height": 1216
    },
    {
        "name": "sam_flirty",
        "sub": "tomboy",
        "prompt": "score_9, score_8_up, score_7_up, score_6_up, score_5_up, score_4_up, rating_safe, 1girl, ashleigh, photorealistic, photograph, 85mm lens, f/1.8, softbox studio lighting, highly detailed skin texture, sharp focus, portrait, photo of an attractive woman, seductive flirty smile, sleek black top",
        "lora_char": "MisterMR_Ashleigh_Phoenix_Pony.safetensors",
        "width": 832,
        "height": 1216
    },
    {
        "name": "keyleth_neutral",
        "sub": "tomboy",
        "prompt": "score_9, score_8_up, score_7_up, score_6_up, score_5_up, score_4_up, rating_safe, 1girl, madeleine, photorealistic, photograph, 85mm lens, f/1.8, softbox studio lighting, highly detailed skin texture, sharp focus, portrait, photo of a glamorous elegant woman, long dark hair, luxury blazer dress",
        "lora_char": "MisterMR_Madeleine_Costa_Pony.safetensors",
        "width": 832,
        "height": 1216
    },
    {
        "name": "keyleth_seductive",
        "sub": "tomboy",
        "prompt": "score_9, score_8_up, score_7_up, score_6_up, score_5_up, score_4_up, rating_safe, 1girl, madeleine, photorealistic, photograph, 85mm lens, f/1.8, softbox studio lighting, highly detailed skin texture, sharp focus, portrait, photo of a glamorous elegant woman, long dark hair, seductive intense gaze, black silk evening dress",
        "lora_char": "MisterMR_Madeleine_Costa_Pony.safetensors",
        "width": 832,
        "height": 1216
    },
    {
        "name": "cg_sam",
        "sub": "cg",
        "prompt": "score_9, score_8_up, score_7_up, score_6_up, score_5_up, score_4_up, rating_explicit, 1girl, 1boy, ashleigh, photorealistic, photograph, 85mm lens, f/1.8, highly detailed skin texture, sharp focus, visual novel 18+ intimate romantic CG scene, handsome man and attractive woman resting close together on a plush leather sofa in a luxury modern VIP suite, warm romantic dim lighting",
        "lora_char": "MisterMR_Ashleigh_Phoenix_Pony.safetensors",
        "width": 1216,
        "height": 832
    },
    {
        "name": "cg_keyleth",
        "sub": "cg",
        "prompt": "score_9, score_8_up, score_7_up, score_6_up, score_5_up, score_4_up, rating_explicit, 1girl, 1boy, madeleine, photorealistic, photograph, 85mm lens, f/1.8, highly detailed skin texture, sharp focus, visual novel 18+ intimate romantic CG scene, handsome man and glamorous elegant woman in silk dress sharing an intimate moment on a balcony overlooking city lights at night",
        "lora_char": "MisterMR_Madeleine_Costa_Pony.safetensors",
        "width": 1216,
        "height": 832
    }
]

def queue_prompt(item):
    nodes = {
        "4": {
            "inputs": {
                "ckpt_name": "ponyDiffusionV6XL_v6StartWithThisOne.safetensors"
            },
            "class_type": "CheckpointLoaderSimple"
        },
        "12": {
            "inputs": {
                "stop_at_clip_layer": -2,
                "clip": ["4", 1]
            },
            "class_type": "CLIPSetLastLayer"
        },
        "10": {
            "inputs": {
                "lora_name": "reakaaka_ponyv7_enhancement_bundle_v0.1.3.safetensors",
                "strength_model": 0.6,
                "strength_clip": 0.6,
                "model": ["4", 0],
                "clip": ["12", 0]
            },
            "class_type": "LoraLoader"
        }
    }
    
    last_model = ["10", 0]
    last_clip = ["10", 1]
    
    if item["lora_char"]:
        nodes["11"] = {
            "inputs": {
                "lora_name": item["lora_char"],
                "strength_model": 0.75,
                "strength_clip": 0.75,
                "model": last_model,
                "clip": last_clip
            },
            "class_type": "LoraLoader"
        }
        last_model = ["11", 0]
        last_clip = ["11", 1]

    nodes["5"] = {
        "inputs": {
            "width": item["width"],
            "height": item["height"],
            "batch_size": 1
        },
        "class_type": "EmptyLatentImage"
    }

    nodes["6"] = {
        "inputs": {
            "text": item["prompt"],
            "clip": last_clip
        },
        "class_type": "CLIPTextEncode"
    }

    nodes["7"] = {
        "inputs": {
            "text": "score_4, score_3_up, (3d, cgi:1.2), cartoon, anime, drawing, illustration, sketch, render, lowres, bad quality, blurry",
            "clip": last_clip
        },
        "class_type": "CLIPTextEncode"
    }

    nodes["3"] = {
        "inputs": {
            "seed": 99999 + hash(item["name"]) % 50000,
            "steps": 30,
            "cfg": 6.0,
            "sampler_name": "euler_ancestral",
            "scheduler": "karras",
            "denoise": 1.0,
            "model": last_model,
            "positive": ["6", 0],
            "negative": ["7", 0],
            "latent_image": ["5", 0]
        },
        "class_type": "KSampler"
    }

    nodes["8"] = {
        "inputs": {
            "samples": ["3", 0],
            "vae": ["4", 2]
        },
        "class_type": "VAEDecode"
    }

    nodes["9"] = {
        "inputs": {
            "filename_prefix": f"pony_pro_real_{item['name']}",
            "images": ["8", 0]
        },
        "class_type": "SaveImage"
    }

    req = urllib.request.Request(f"{COMFY_URL}/prompt", data=json.dumps({"prompt": nodes}).encode("utf-8"), headers={"Content-Type": "application/json"})
    res = json.loads(urllib.request.urlopen(req).read())
    prompt_id = res["prompt_id"]
    print(f"Queued {item['name']} (Pony XL CLIP-Skip-2 + {item['lora_char'] or 'Base'}) -> ID: {prompt_id}")
    
    while True:
        time.sleep(2)
        h_req = urllib.request.urlopen(f"{COMFY_URL}/history/{prompt_id}")
        h_data = json.loads(h_req.read())
        if prompt_id in h_data:
            out = h_data[prompt_id]["outputs"]["9"]["images"][0]["filename"]
            src_path = os.path.join(r"C:\Users\usuario\Documents\ComfyUI_core\output", out)
            target_png = os.path.join(DEST_DIR, item["sub"], f"{item['name']}.png")
            target_jpg = os.path.join(DEST_DIR, item["sub"], f"{item['name']}.jpg")
            os.makedirs(os.path.dirname(target_png), exist_ok=True)
            shutil.copyfile(src_path, target_png)
            shutil.copyfile(src_path, target_jpg)
            print(f"Generated & Copied PRO {item['name']} to {target_png}")
            break

def main():
    print("Generating official high-quality photorealistic Pony XL images (CLIP Skip -2, Karras, Full Score Tags)...")
    for item in prompts:
        queue_prompt(item)
    print("ALL OFFICIAL PRO PONY REALISTIC ASSETS GENERATED SUCCESSFULLY!")

if __name__ == "__main__":
    main()
