import json
import os
import shutil
import time
import urllib.request

COMFY_URL = "http://127.0.0.1:8188"
OUTPUT_DIR = r"demos\summoner-emporium\data\assets\moody_gallery"

images_def = [
    {
        "filename": "moody_01_boutique.png",
        "prompt": "moody style, a breathtaking photorealistic portrait of an attractive 23 year old French woman, sharp focus on face, highly detailed eyes and face, highly detailed skin texture, warm moody cinematic lighting, soft focus background, sleek black dress, inside a luxury modern boutique at night, 8k resolution, raw photo",
        "seed": 987651
    },
    {
        "filename": "moody_02_penthouse.png",
        "prompt": "moody style, a breathtaking photorealistic portrait of a gorgeous 25 year old Latina woman, sharp focus on face, highly detailed eyes and face, highly detailed skin texture, warm moody cinematic lighting, soft focus background, silk evening gown, standing on a penthouse balcony with city night lights, 8k resolution, raw photo",
        "seed": 987652
    },
    {
        "filename": "moody_03_cozy_cafe.png",
        "prompt": "moody style, a breathtaking photorealistic portrait of a charming 22 year old Japanese woman, sharp focus on face, highly detailed eyes and face, highly detailed skin texture, warm golden hour lighting, soft focus background, oversized knitted sweater, holding a cappuccino mug, 8k resolution, raw photo",
        "seed": 987653
    },
    {
        "filename": "moody_04_athletic.png",
        "prompt": "moody style, a breathtaking photorealistic portrait of an athletic 24 year old Scandinavian woman, sharp focus on face, highly detailed eyes and face, highly detailed skin texture, warm moody cinematic lighting, soft focus background, stylish athletic wear, confident smile, modern gym lounge, 8k resolution, raw photo",
        "seed": 987654
    },
    {
        "filename": "moody_05_cyberpunk_street.png",
        "prompt": "moody style, a breathtaking photorealistic portrait of an attractive 21 year old tomboy woman, sharp focus on face, highly detailed eyes and face, highly detailed skin texture, glowing neon lighting, soft focus background, stylish leather jacket, standing on a rain-slicked city street at night, 8k resolution, raw photo",
        "seed": 987655
    },
    {
        "filename": "moody_06_botanical_garden.png",
        "prompt": "moody style, a breathtaking photorealistic portrait of a serene 23 year old Italian woman with wavy hair, sharp focus on face, highly detailed eyes and face, highly detailed skin texture, soft fairy lights, soft focus background, summer linen shirt, greenhouse garden at night, 8k resolution, raw photo",
        "seed": 987656
    },
    {
        "filename": "moody_07_sports_car.png",
        "prompt": "moody style, a breathtaking photorealistic portrait of a stylish 26 year old woman, sharp focus on face, highly detailed eyes and face, highly detailed skin texture, LED interior car lighting, soft focus background, sitting inside a luxury sports car, looking towards camera, 8k resolution, raw photo",
        "seed": 987657
    },
    {
        "filename": "moody_08_golden_sunset_beach.png",
        "prompt": "moody style, a breathtaking photorealistic portrait of a beautiful 22 year old woman, sharp focus on face, highly detailed eyes and face, highly detailed skin texture, golden hour sunset lighting, soft focus background, tropical beach breeze, ocean horizon, 8k resolution, raw photo",
        "seed": 987658
    },
    {
        "filename": "moody_09_art_studio.png",
        "prompt": "moody style, a breathtaking photorealistic portrait of a creative 24 year old woman, sharp focus on face, highly detailed eyes and face, highly detailed skin texture, warm sunlight through window, soft focus background, painter loft studio, paint-splattered shirt, holding paintbrush, 8k resolution, raw photo",
        "seed": 987659
    },
    {
        "filename": "moody_10_vip_party.png",
        "prompt": "moody style, a breathtaking photorealistic portrait of a gorgeous 25 year old woman in a VIP party, sharp focus on face, highly detailed eyes and face, highly detailed skin texture, warm moody cinematic lighting, soft focus background, holding a champagne glass, elegant evening dress, 8k resolution, raw photo",
        "seed": 987660
    }
]

def queue_image(item):
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
                "text": "blurry face, out of focus, blurry eyes, distorted face, low quality, cartoon, 3d render, lowres",
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
                "filename_prefix": f"moody_crisp_{item['filename'][:-4]}",
                "images": ["9", 0]
            },
            "class_type": "SaveImage"
        }
    }

    req = urllib.request.Request(f"{COMFY_URL}/prompt", data=json.dumps({"prompt": workflow}).encode("utf-8"), headers={"Content-Type": "application/json"})
    res = json.loads(urllib.request.urlopen(req).read())
    prompt_id = res["prompt_id"]
    print(f"Queued {item['filename']} -> ID: {prompt_id}")

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
            print(f"Generated & Saved CRISP: {item['filename']} -> {target_path}")
            break

def main():
    print("Regenerating all 10 Moody Z-Image photos with crisp sharp face focus master formula...")
    for idx, item in enumerate(images_def, 1):
        print(f"[{idx}/10] Regenerating {item['filename']}...")
        queue_image(item)
    print("ALL 10 CRISP MOODY Z-IMAGE PHOTOS GENERATED SUCCESSFULLY!")

if __name__ == "__main__":
    main()
