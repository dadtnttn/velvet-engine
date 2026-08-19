import json
import os
import shutil
import time
import urllib.request

COMFY_URL = "http://127.0.0.1:8188"
OUTPUT_DIR = r"demos\summoner-emporium\data\assets\moody_gallery"

item = {
    "filename": "moody_10_vip_party_crisp.png",
    "prompt": "moody style, a breathtaking photorealistic portrait of a gorgeous 25 year old woman in a VIP lounge, sharp focus on face, highly detailed eyes and face, holding a crystal champagne flute, warm moody cinematic lighting, 85mm lens photo, f/1.8, highly detailed skin texture, 8k resolution, raw photo",
    "seed": 987654
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
            "shift": 3.0,
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
            "width": 832,
            "height": 1216,
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
            "text": "blurry face, out of focus, distorted face, blurry eyes, low quality, cartoon, 3d render, lowres",
            "clip": ["3", 0]
        },
        "class_type": "CLIPTextEncode"
    },
    "8": {
        "inputs": {
            "seed": item["seed"],
            "steps": 12,
            "cfg": 3.8,
            "sampler_name": "dpmpp_2m_sde",
            "scheduler": "beta",
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
            "filename_prefix": "moody_crisp_test",
            "images": ["9", 0]
        },
        "class_type": "SaveImage"
    }
}

def main():
    print("Testing crisp sharp face prompt formula...")
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
            print(f"CRISP TEST PHOTO GENERATED SUCCESSFULLY! Saved to {target_path}")
            break

if __name__ == "__main__":
    main()
