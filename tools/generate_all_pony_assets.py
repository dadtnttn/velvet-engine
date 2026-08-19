import json
import os
import shutil
import time
import urllib.request

COMFY_URL = "http://127.0.0.1:8188"
DEST_DIR = r"demos\summoner-emporium\data\assets"

prompts_def = [
    {
        "name": "store",
        "sub": "backgrounds/png",
        "prompt": "score_9, score_8_up, score_7_up, realistic photorealistic modern luxury boutique store interior, glass display cases, glowing neon accents, polished dark marble floors, premium luxury items on display, dramatic ambient lighting, 8k photographic quality",
        "lora": None,
        "width": 1024,
        "height": 576
    },
    {
        "name": "altar",
        "sub": "backgrounds/png",
        "prompt": "score_9, score_8_up, score_7_up, realistic photorealistic modern secret summoning chamber inside a high-tech penthouse, glowing purple holographic summoning ring on a black glass floor, levitating glass sphere, neon blue background highlights, sleek cinematic lighting, 8k resolution",
        "lora": None,
        "width": 1024,
        "height": 576
    },
    {
        "name": "vip",
        "sub": "backgrounds/png",
        "prompt": "score_9, score_8_up, score_7_up, realistic photorealistic luxury modern VIP lounge penthouse suite, plush leather sofa, warm ambient mood lighting, panoramic city night view through floor-to-ceiling glass windows, champagne glasses on marble table, romantic atmosphere, 8k photographic quality",
        "lora": None,
        "width": 1024,
        "height": 576
    },
    {
        "name": "sam_neutral",
        "sub": "tomboy",
        "prompt": "score_9, score_8_up, score_7_up, 1girl, realistic photorealistic portrait of an attractive tomboy woman with short dark hair, stylish leather jacket, confident friendly expression, studio lighting, 8k resolution",
        "lora": "keyleth-ponyxl-lora-nochekaiser.safetensors",
        "width": 768,
        "height": 1024
    },
    {
        "name": "sam_flirty",
        "sub": "tomboy",
        "prompt": "score_9, score_8_up, score_7_up, 1girl, realistic photorealistic portrait of an attractive tomboy woman with short dark hair, seductive flirty smile, stylish black top, warm romantic dim lighting, 8k resolution",
        "lora": "keyleth-ponyxl-lora-nochekaiser.safetensors",
        "width": 768,
        "height": 1024
    },
    {
        "name": "keyleth_neutral",
        "sub": "tomboy",
        "prompt": "score_9, score_8_up, score_7_up, 1girl, realistic photorealistic portrait of a glamorous elegant woman, long dark hair, sophisticated blazer dress, luxury penthouse interior background, 8k resolution",
        "lora": "keyleth-ponyxl-lora-nochekaiser.safetensors",
        "width": 768,
        "height": 1024
    },
    {
        "name": "keyleth_seductive",
        "sub": "tomboy",
        "prompt": "score_9, score_8_up, score_7_up, 1girl, realistic photorealistic portrait of a glamorous elegant woman, long dark hair, seductive intense gaze, black silk evening dress, warm moody ambient lighting, 8k resolution",
        "lora": "keyleth-ponyxl-lora-nochekaiser.safetensors",
        "width": 768,
        "height": 1024
    },
    {
        "name": "cg_sam",
        "sub": "cg",
        "prompt": "score_9, score_8_up, score_7_up, 1girl, 1boy, realistic visual novel 18+ romantic CG scene, handsome man and attractive tomboy woman resting intimately close on a plush leather sofa in a luxury modern VIP suite, warm romantic dim lighting, 8k resolution",
        "lora": None,
        "width": 1024,
        "height": 576
    },
    {
        "name": "cg_keyleth",
        "sub": "cg",
        "prompt": "score_9, score_8_up, score_7_up, 1girl, 1boy, realistic visual novel 18+ romantic CG scene, handsome man and glamorous elegant woman sharing an intimate moment on a balcony overlooking city lights at night, silk dress, 8k resolution",
        "lora": None,
        "width": 1024,
        "height": 576
    }
]

def queue_gen(item):
    workflow = {
        '3': {
            'inputs': {
                'seed': 100 + hash(item['name']) % 10000,
                'steps': 20,
                'cfg': 7.0,
                'sampler_name': 'euler_ancestral',
                'scheduler': 'normal',
                'denoise': 1.0,
                'model': ['4', 0],
                'positive': ['6', 0],
                'negative': ['7', 0],
                'latent_image': ['5', 0]
            },
            'class_type': 'KSampler'
        },
        '4': {
            'inputs': {
                'ckpt_name': 'ponyDiffusionV6XL_v6StartWithThisOne.safetensors'
            },
            'class_type': 'CheckpointLoaderSimple'
        },
        '5': {
            'inputs': {
                'width': item['width'],
                'height': item['height'],
                'batch_size': 1
            },
            'class_type': 'EmptyLatentImage'
        },
        '6': {
            'inputs': {
                'text': item['prompt'],
                'clip': ['4', 1]
            },
            'class_type': 'CLIPTextEncode'
        },
        '7': {
            'inputs': {
                'text': 'score_4, score_3_up, 3d, blurry, worst quality, low quality, cartoon',
                'clip': ['4', 1]
            },
            'class_type': 'CLIPTextEncode'
        },
        '8': {
            'inputs': {
                'samples': ['3', 0],
                'vae': ['4', 2]
            },
            'class_type': 'VAEDecode'
        },
        '9': {
            'inputs': {
                'filename_prefix': f"pony_asset_{item['name']}",
                'images': ['8', 0]
            },
            'class_type': 'SaveImage'
        }
    }
    
    req = urllib.request.Request(f"{COMFY_URL}/prompt", data=json.dumps({'prompt': workflow}).encode('utf-8'), headers={'Content-Type': 'application/json'})
    res = json.loads(urllib.request.urlopen(req).read())
    prompt_id = res['prompt_id']
    print(f"Queued {item['name']} -> prompt_id: {prompt_id}")
    
    # Wait for completion
    while True:
        time.sleep(2)
        h_req = urllib.request.urlopen(f"{COMFY_URL}/history/{prompt_id}")
        h_data = json.loads(h_req.read())
        if prompt_id in h_data:
            out = h_data[prompt_id]['outputs']['9']['images'][0]['filename']
            src_path = os.path.join(r"C:\Users\usuario\Documents\ComfyUI_core\output", out)
            target_path = os.path.join(DEST_DIR, item['sub'], f"{item['name']}.png")
            os.makedirs(os.path.dirname(target_path), exist_ok=True)
            shutil.copyfile(src_path, target_path)
            print(f"Generated and copied {item['name']} to {target_path}")
            break

def main():
    print("Starting generation of assets with local Pony Diffusion V6 XL model...")
    for item in prompts_def:
        queue_gen(item)
    print("All Pony assets generated successfully!")

if __name__ == "__main__":
    main()
