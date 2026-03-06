"""Provider abstraction for image generation via browser automation.

Each provider module must export:
  URL              — str, the base URL to navigate to
  SELECTORS        — dict of DOM selectors used by JS snippets
  RATE_LIMIT_COOLDOWN — int, seconds to wait on rate limit

  fill_prompt_js(text: str) -> str   — JS to inject prompt text
  click_send_js() -> str             — JS to click the send button
  image_check_js() -> str            — JS to count generated images (returns count)
  image_loaded_js() -> str           — JS to check if first image is fully loaded
  image_src_js() -> str              — JS to extract first generated image URL
  rate_limit_check_js() -> str       — JS to detect rate limiting
  upload_reference_js(b64: str, filename: str) -> str — JS to upload via DataTransfer
"""

import importlib

PROVIDERS = {
    "chatgpt": "providers.chatgpt",
    "gemini": "providers.gemini",
}


def get_provider(name: str):
    """Return the provider module by name."""
    if name not in PROVIDERS:
        raise ValueError(f"Unknown provider '{name}'. Available: {', '.join(PROVIDERS)}")
    return importlib.import_module(f".{name}", package="providers")


def base64_upload_js(b64: str, filename: str = "reference.png",
                     file_input_selector: str = 'input[type="file"]') -> str:
    """Shared JS for injecting a base64-encoded file into a file input via DataTransfer API.

    Works for both ChatGPT and Gemini — just pass different file_input_selector.
    """
    return f'''
(function() {{
    var b64 = "{b64}";
    var byteChars = atob(b64);
    var byteArray = new Uint8Array(byteChars.length);
    for (var i = 0; i < byteChars.length; i++) {{
        byteArray[i] = byteChars.charCodeAt(i);
    }}
    var blob = new Blob([byteArray], {{type: "image/png"}});
    var file = new File([blob], "{filename}", {{type: "image/png"}});
    var dt = new DataTransfer();
    dt.items.add(file);
    var fileInput = document.querySelector('{file_input_selector}');
    if (fileInput) {{
        fileInput.files = dt.files;
        fileInput.dispatchEvent(new Event("change", {{bubbles: true}}));
        return "uploaded";
    }}
    return "no_file_input";
}})()
'''
