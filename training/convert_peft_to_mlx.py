#!/usr/bin/env python3
"""
Convert PEFT LoRA adapter (safetensors) to mlx-lm LoRA adapter format.

PEFT format:
  Keys: base_model.model.model.layers.{i}.{module}.lora_{A|B}.weight
  File: adapter_model.safetensors or model.safetensors

mlx-lm format:
  Keys: language_model.model.layers.{i}.{module}.lora_{a|b}
  File: adapters.safetensors
  Config: adapter_config.json with {fine_tune_type, num_layers, lora_parameters}
"""
import json
import os
import sys
import numpy as np

def convert_peft_to_mlx(
    peft_dir: str,
    output_dir: str,
    num_model_layers: int = 40,
    add_prefix: str = "language_model.",
):
    from safetensors import safe_open
    from safetensors.numpy import save_file as np_save_file

    # Find the safetensors file
    for name in ["model.safetensors", "adapter_model.safetensors"]:
        path = os.path.join(peft_dir, name)
        if os.path.exists(path):
            sf_path = path
            break
    else:
        raise FileNotFoundError(f"No safetensors file found in {peft_dir}")

    # Read PEFT config
    with open(os.path.join(peft_dir, "adapter_config.json")) as f:
        peft_config = json.load(f)

    rank = peft_config["r"]
    alpha = peft_config["lora_alpha"]
    dropout = peft_config.get("lora_dropout", 0.0)
    target_modules = peft_config["target_modules"]
    scale = alpha / rank

    print(f"PEFT config: r={rank}, alpha={alpha}, scale={scale:.4f}, dropout={dropout}")
    print(f"Target modules: {target_modules}")

    # Convert weights
    mlx_weights = {}
    with safe_open(sf_path, framework="numpy") as f:
        peft_keys = sorted(f.keys())
        print(f"Total PEFT keys: {len(peft_keys)}")

        for key in peft_keys:
            tensor = f.get_tensor(key)

            # Strip PEFT prefix: base_model.model. -> ""
            new_key = key
            if new_key.startswith("base_model.model."):
                new_key = new_key[len("base_model.model."):]

            # Add multimodal prefix
            new_key = add_prefix + new_key

            # Rename lora_A.weight -> lora_a, lora_B.weight -> lora_b
            # PEFT uses F.linear(x, W) = x @ W.T, so weights are transposed vs MLX
            # MLX computes x @ lora_a @ lora_b directly, so we transpose both
            new_key = new_key.replace(".lora_A.weight", ".lora_a")
            new_key = new_key.replace(".lora_B.weight", ".lora_b")

            tensor = tensor.T  # PEFT (r, in) -> MLX (in, r) for A; PEFT (out, r) -> MLX (r, out) for B

            mlx_weights[new_key] = tensor
            if "layers.0." in new_key:
                print(f"  {key} -> {new_key}  shape={tensor.shape}")

    print(f"\nConverted {len(mlx_weights)} tensors")

    # Determine which layers have LoRA
    lora_layers = set()
    for key in mlx_weights:
        parts = key.split(".")
        for i, p in enumerate(parts):
            if p == "layers" and i + 1 < len(parts):
                try:
                    lora_layers.add(int(parts[i + 1]))
                except ValueError:
                    pass
    num_lora_layers = len(lora_layers)
    print(f"LoRA applied to {num_lora_layers} of {num_model_layers} layers")

    # Determine key patterns
    key_patterns = set()
    for key in mlx_weights:
        # Extract the module path between layer index and lora_a/b
        parts = key.split(".")
        for i, p in enumerate(parts):
            if p == "layers":
                # Find the lora_a/b suffix
                rest = ".".join(parts[i + 2:])
                module = rest.rsplit(".lora_", 1)[0]
                key_patterns.add(module)
                break
    print(f"Module patterns: {sorted(key_patterns)}")

    # Save mlx adapter weights
    os.makedirs(output_dir, exist_ok=True)
    out_sf = os.path.join(output_dir, "adapters.safetensors")
    np_save_file(mlx_weights, out_sf)
    print(f"Saved weights to {out_sf}")

    # Create mlx-lm adapter_config.json
    mlx_config = {
        "fine_tune_type": "lora",
        "num_layers": num_lora_layers,
        "lora_parameters": {
            "rank": rank,
            "scale": scale,
            "dropout": dropout,
            "keys": sorted(key_patterns),
        },
    }
    config_path = os.path.join(output_dir, "adapter_config.json")
    with open(config_path, "w") as f:
        json.dump(mlx_config, f, indent=2)
    print(f"Saved config to {config_path}")
    print(json.dumps(mlx_config, indent=2))

    return output_dir

if __name__ == "__main__":
    peft_dir = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(__file__), "lora_checkpoints")
    output_dir = sys.argv[2] if len(sys.argv) > 2 else os.path.join(os.path.dirname(__file__), "mlx_adapter")

    convert_peft_to_mlx(peft_dir, output_dir)
