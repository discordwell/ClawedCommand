"""ChatGPT image generation provider — DOM selectors and JS snippets.

Extracted from batch_remaining.py. These work with ChatGPT's current DOM
as of March 2026. Claude uses these via MCP browser tools (javascript_tool).
"""

from .base import base64_upload_js

URL = "https://chatgpt.com/"

SELECTORS = {
    "prompt_textarea": "#prompt-textarea",
    "send_button": 'button[data-testid="send-button"]',
    "send_button_alt": 'button[aria-label="Send prompt"]',
    "generated_image": 'img[alt="Generated image"]',
    "file_input": 'input[type="file"]',
    "add_files_button": 'button[aria-label="Add files and more"]',
}

RATE_LIMIT_COOLDOWN = 300  # 5 minutes


def fill_prompt_js(text: str) -> str:
    """JS to fill ChatGPT's ProseMirror textarea with prompt text.

    Uses execCommand("insertText") which integrates with ProseMirror's
    internal state. innerHTML silently fails (React/ProseMirror ignores it).
    """
    escaped = text.replace("\\", "\\\\").replace("`", "\\`").replace("${", "\\${")
    return f'''
(function() {{
    var textarea = document.querySelector("#prompt-textarea");
    if (!textarea) return "no_textarea";
    textarea.focus();
    document.execCommand("selectAll", false, null);
    document.execCommand("delete", false, null);
    document.execCommand("insertText", false, `{escaped}`);
    textarea.dispatchEvent(new Event("input", {{bubbles: true}}));
    return "filled";
}})()
'''


def click_send_js() -> str:
    """JS to click the send button."""
    return '''
(function() {
    var btn = document.querySelector('button[data-testid="send-button"]')
           || document.querySelector('button[aria-label="Send prompt"]');
    if (btn) { btn.click(); return "sent"; }
    return "no_button";
})()
'''


def image_check_js() -> str:
    """JS to count generated images in the response."""
    return '''
document.querySelectorAll('img[alt="Generated image"]').length.toString()
'''


def image_loaded_js() -> str:
    """JS to check if the first generated image is fully loaded."""
    return '''
(function() {
    var img = document.querySelector('img[alt="Generated image"]');
    if (!img) return "no_img";
    if (img.complete && img.naturalWidth > 0) return "loaded";
    return "loading";
})()
'''


def image_src_js() -> str:
    """JS to extract the src URL of the first generated image."""
    return '''
(function() {
    var imgs = document.querySelectorAll('img[alt="Generated image"]');
    if (imgs.length === 0) return "no_images";
    return imgs[0].src;
})()
'''


def rate_limit_check_js() -> str:
    """JS to detect rate limiting messages."""
    return '''
(function() {
    var body = document.body ? document.body.innerText : "";
    if (body.match(/rate limit|too many|try again later|usage cap/i)) return "rate_limited";
    return "ok";
})()
'''


def upload_reference_js(b64: str, filename: str = "style_reference.png") -> str:
    """JS to upload a reference image via DataTransfer into ChatGPT's file input.

    Call click_add_files_js() first to open the file input, then this.
    """
    return base64_upload_js(b64, filename, SELECTORS["file_input"])


def click_add_files_js() -> str:
    """JS to click the 'Add files and more' button before uploading."""
    return '''
(function() {
    var addBtn = document.querySelector('button[aria-label="Add files and more"]');
    if (addBtn) { addBtn.click(); return "clicked"; }
    return "no_button";
})()
'''


def new_chat_js() -> str:
    """JS to navigate to a new chat."""
    return 'window.location.href = "https://chatgpt.com/"'


def check_ready_js() -> str:
    """JS to check if the prompt textarea is ready."""
    return '''
document.querySelector("#prompt-textarea") ? "ready" : "loading"
'''
