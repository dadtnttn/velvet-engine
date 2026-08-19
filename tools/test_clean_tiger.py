import json
import os
import shutil
import time
import urllib.request

COMFY_URL = "http://127.0.0.1:8188"
OUTPUT_DIR = r"demos\summoner-emporium\data\assets\moody_gallery"

item = {
    "filename": "clean_zoo_tiger_no_glare.png",
    "prompt": "a clean realistic outdoor photograph of a single Bengal tiger in a zoo enclosure, natural daylight, sharp focus on tiger face and fur, clean background, 85mm lens photo",
    "seed": 998877
}

workflow = {
    "1": {
        "inputs": {
            "unet_name": "z_image_turbo_bf16.safetensors",
            "weight_dtype": "default"
        },
        "class_type": "UNETLoader"
    },
    "2": {
        "inputs": {
            "shift": 1.0,
            "model": ["1", 0]
        },
        "class_type": "ModelSamplingAuraFlow"
    },
    "3": {
        "inputs": {
            "clip_name": "qwen_3_4b.safetensors",
            "type": "lumina2",
            "device": "default"
        },
        "class_type": "CLIPLoader"
    },
    "4": {
        "inputs": {
            "vae_name": "ae.safetensors"
        },
        "class_type": "VAELoader"
    },
    "5": {
        "inputs": {
            "width": 1216,
            "height": 832,
            "batch_size": 1
        },
        "class_type": "EmptyLatentImage"
    },
    "6": {
        "inputs": {
            "text": item["prompt"],
            "clip": ["3", 0]
        },
        "class_type": "CLIPTextEncode"
    },
    "7": {
        "inputs": {
            "text": "rainbow reflections, light leaks, chromatic aberration, colorful glare, neon artifacts, lens flare, colorful spots, artifacts, glares, distorted tiger, blurry, low quality, 3d render, cartoon",
            "clip": ["3", 0]
        },
        "class_type": "CLIPTextEncode"
    },
    "8": {
        "inputs": {
            "seed": item["seed"],
            "steps": 10,
            "cfg": 2.0,
            "sampler_name": "euler",
            "scheduler": "normal",
            "denoise": 1.0,
            "model": ["2", 0],
            "positive": ["6", 0],
            "negative": ["7", 0],
            "latent_image": ["5", 0]
        },
        "class_type": "KSampler"
    },
    "9": {
        "inputs": {
            "samples": ["8", 0],
            "vae": ["4", 0]
        },
        "class_type": "VAEDecode"
    },
    "10": {
        "inputs": {
            "filename_prefix": "clean_no_glare_tiger",
            "images": ["9", 0]
        },
        "class_type": "SaveImage"
    }
}

def main():
    print("Testing clean zoo tiger photo without glare / rainbow artifacts...")
    req = urllib.request.Request(f"{COMFY_URL}/prompt", data=json.dumps({"prompt": workflow}).encode("utf-8"), headers={"Content-Type": "application/json"})
    res = json.loads(urllib.request.urlopen(req).read())
    prompt_id = res["prompt_id"]
    print(f"Queued -> ID: {prompt_id}")

    while True:
        time.sleep(2)
        h_req = urllib.request.urlopen(f"{COMFY_URL}/history/{prompt_id}")
        h_data = json.loads(h_req.read())
        if prompt_id in h_data:
            out = h_data[prompt_id]["outputs"]["10"]["images"][0]["filename"]
            src_path = os.path.join(r"C:\Users\usuario\Documents\ComfyUI_core\output", out)
            target_path = os.path.join(OUTPUT_DIR, item["filename"])
            os.makedirs(os.path.dirname(target_path), exist_ok=True)
            shutil.copyfile(src_path, target_path)
            print(f"CLEAN TIGER PHOTO GENERATED! Saved to {target_path}")
            break

if __name__ == "__main__":
    main()
