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
        "prompt": "moody style, a breathtaking photorealistic portrait of an elegant 23 year old French woman, sleek black dress, inside a luxury modern boutique at night, glowing neon highlights, soft skin texture, 8k resolution, dslr photo",
        "seed": 10001
    },
    {
        "filename": "moody_02_penthouse.png",
        "prompt": "moody style, a stunning photorealistic photo of a glamorous 25 year old Latina woman, silk evening gown, standing on a penthouse balcony overlooking a glowing city night skyline, warm ambient lighting, highly detailed, 8k resolution",
        "seed": 20002
    },
    {
        "filename": "moody_03_cozy_cafe.png",
        "prompt": "moody style, a cozy portrait of a charming 22 year old Japanese woman, oversized knitted sweater, sitting inside an aesthetic coffee shop, holding a cappuccino mug, warm golden hour light, 8k resolution, raw photo",
        "seed": 30003
    },
    {
        "filename": "moody_04_athletic.png",
        "prompt": "moody style, a high-fashion portrait of an athletic 24 year old Scandinavian woman, stylish sports attire, confident smile, neon-lit modern gym lounge, detailed skin texture, sharp focus, 8k resolution",
        "seed": 40004
    },
    {
        "filename": "moody_05_cyberpunk_street.png",
        "prompt": "moody style, a cool portrait of a 21 year old tomboy woman, leather jacket, standing on a rain-slicked city street at night under glowing purple and cyan neon signs, bokeh reflections, 8k resolution photo",
        "seed": 50005
    },
    {
        "filename": "moody_06_botanical_garden.png",
        "prompt": "moody style, a serene portrait of a 23 year old Italian woman with wavy brown hair, summer linen shirt, surrounded by exotic plants in a greenhouse with fairy lights, soft dreamy lighting, 8k resolution",
        "seed": 60006
    },
    {
        "filename": "moody_07_sports_car.png",
        "prompt": "moody style, a dramatic photo of a stylish 26 year old woman sitting inside a luxury sports car, interior ambient LED lighting, looking towards the camera, high resolution, 8k raw photo",
        "seed": 70007
    },
    {
        "filename": "moody_08_golden_sunset_beach.png",
        "prompt": "moody style, a beautiful portrait of a 22 year old woman on a tropical beach during golden hour sunset, ocean breeze, soft sun rays reflecting on skin, cinematic lighting, 8k resolution photo",
        "seed": 80008
    },
    {
        "filename": "moody_09_art_studio.png",
        "prompt": "moody style, an artistic portrait of a creative 24 year old woman in an artist loft studio, paint-splattered shirt, holding an oil paintbrush, warm sunlight through large window, detailed portrait, 8k resolution",
        "seed": 90009
    },
    {
        "filename": "moody_10_vip_party.png",
        "prompt": "moody style, a luxurious party photo of a gorgeous 25 year old woman holding a crystal champagne flute, VIP lounge club, golden bokeh lights background, glamorous smile, 8k resolution raw photo",
        "seed": 100010
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
                "text": "blurry, low quality, distorted, cartoon, 3d render, lowres",
                "clip": ["3", 0]
            },
            "class_type": "CLIPTextEncode"
        },
        "8": {
            "inputs": {
                "seed": item["seed"],
                "steps": 10,
                "cfg": 3.5,
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
                "filename_prefix": f"moody_batch_{item['filename'][:-4]}",
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
            print(f"Generated & Saved: {item['filename']} -> {target_path}")
            break

def main():
    print("Generating batch of 10 unique Moody Z-Image Turbo photorealistic images...")
    for idx, item in enumerate(images_def, 1):
        print(f"[{idx}/10] Generating {item['filename']}...")
        queue_image(item)
    print("ALL 10 MOODY Z-IMAGE PHOTOS GENERATED SUCCESSFULLY!")

if __name__ == "__main__":
    main()
