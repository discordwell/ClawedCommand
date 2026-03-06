"""Gemini image generation provider — DOM selectors and JS snippets.

Gemini's DOM changes frequently. When hardcoded selectors fail, Claude should
use MCP read_page/find tools to discover current selectors and update this file.

Initial selectors based on gemini.google.com/app as of March 2026.
"""

from .base import base64_upload_js

URL = "https://gemini.google.com/app"

# Gemini's DOM uses dynamic class names, so we rely on structural selectors
# and data attributes where possible. These are best-effort starting points.
SELECTORS = {
    # The main text input area (contenteditable div)
    "prompt_input": '.ql-editor[contenteditable="true"], div[contenteditable="true"][role="textbox"]',
    # Send button — Gemini uses a mat-icon-button with send icon
    "send_button": 'button[aria-label="Send message"], button.send-button',
    # Generated images in model responses — large images (>200px) in response containers
    "response_image": '.response-container img, .model-response img',
    # File input for uploads
    "file_input": 'input[type="file"]',
    # Upload/attach button
    "attach_button": 'button[aria-label="Upload file"], button[aria-label="Add image"]',
}

RATE_LIMIT_COOLDOWN = 60  # Gemini rate limits are shorter

# When selectors fail, Claude should use these hints with MCP find/read_page
# to discover current selectors dynamically.
SELECTOR_DISCOVERY_HINTS = {
    "prompt_input": "Look for a contenteditable div or textarea near the bottom of the page where the user types messages",
    "send_button": "Look for a send/submit button near the text input area, often a circular button with an arrow icon. NOTE: coordinate-based clicks may miss — use JS .click() via javascript_tool instead",
    "response_image": "Look for large images (>200px) inside the model's response area, not UI icons or avatars",
    "file_input": "Look for a hidden file input element, often triggered by an attach/upload button",
    "attach_button": "Look for an attach/upload/image button near the text input area",
    "download_button": "Gemini has a 'Download full size image' button below generated images. Use find('download image') to locate it. This is the BEST download method — cross-origin fetch and canvas toDataURL both fail",
}


def fill_prompt_js(text: str) -> str:
    """JS to fill Gemini's contenteditable input with prompt text."""
    escaped = text.replace("\\", "\\\\").replace("`", "\\`").replace("${", "\\${")
    return f'''
(function() {{
    var input = document.querySelector('.ql-editor[contenteditable="true"]')
             || document.querySelector('div[contenteditable="true"][role="textbox"]');
    if (!input) return "no_input";
    input.focus();
    document.execCommand("selectAll", false, null);
    document.execCommand("delete", false, null);
    document.execCommand("insertText", false, `{escaped}`);
    input.dispatchEvent(new Event("input", {{bubbles: true}}));
    return "filled";
}})()
'''


def click_send_js() -> str:
    """JS to click the send message button."""
    return '''
(function() {
    var btn = document.querySelector('button[aria-label="Send message"]')
           || document.querySelector('button.send-button')
           || document.querySelector('button[data-testid="send-button"]');
    if (btn && !btn.disabled) { btn.click(); return "sent"; }
    if (btn && btn.disabled) return "button_disabled";
    return "no_button";
})()
'''


def image_check_js() -> str:
    """JS to count large generated images in the model response.

    Filters to images >200px to exclude UI icons, avatars, etc.
    """
    return '''
(function() {
    var imgs = document.querySelectorAll('.response-container img, .model-response img, [class*="response"] img');
    var count = 0;
    for (var i = 0; i < imgs.length; i++) {
        if (imgs[i].naturalWidth > 200 || imgs[i].width > 200) count++;
    }
    return count.toString();
})()
'''


def image_loaded_js() -> str:
    """JS to check if the first large response image is fully loaded."""
    return '''
(function() {
    var imgs = document.querySelectorAll('.response-container img, .model-response img, [class*="response"] img');
    for (var i = 0; i < imgs.length; i++) {
        if (imgs[i].naturalWidth > 200 || imgs[i].width > 200) {
            if (imgs[i].complete && imgs[i].naturalWidth > 0) return "loaded";
            return "loading";
        }
    }
    return "no_img";
})()
'''


def image_src_js() -> str:
    """JS to extract the src URL of the first large generated image."""
    return '''
(function() {
    var imgs = document.querySelectorAll('.response-container img, .model-response img, [class*="response"] img');
    for (var i = 0; i < imgs.length; i++) {
        if (imgs[i].naturalWidth > 200 || imgs[i].width > 200) {
            return imgs[i].src;
        }
    }
    return "no_images";
})()
'''


def rate_limit_check_js() -> str:
    """JS to detect rate limiting or quota messages from Gemini."""
    return '''
(function() {
    var body = document.body ? document.body.innerText : "";
    if (body.match(/quota|rate limit|too many requests|try again later|temporarily unavailable/i))
        return "rate_limited";
    return "ok";
})()
'''


def upload_reference_js(b64: str, filename: str = "style_reference.png") -> str:
    """JS to upload a reference image via DataTransfer into Gemini's file input."""
    return base64_upload_js(b64, filename, SELECTORS["file_input"])


def click_attach_js() -> str:
    """JS to click the attach/upload button before uploading a file."""
    return '''
(function() {
    var btn = document.querySelector('button[aria-label="Upload file"]')
           || document.querySelector('button[aria-label="Add image"]')
           || document.querySelector('button[aria-label="Attach files"]');
    if (btn) { btn.click(); return "clicked"; }
    return "no_button";
})()
'''


def click_download_js() -> str:
    """JS to click Gemini's 'Download full size image' button.

    This is the reliable download method. Cross-origin fetch and canvas
    toDataURL both fail due to CORS restrictions. The download button
    triggers a browser download to ~/Downloads/.

    Use MCP find('download image') as a fallback if this selector breaks.
    """
    return '''
(function() {
    var btns = document.querySelectorAll('button');
    for (var i = 0; i < btns.length; i++) {
        var label = btns[i].getAttribute('aria-label') || btns[i].textContent || '';
        if (label.match(/download.*image/i)) {
            btns[i].click();
            return "downloading";
        }
    }
    return "no_download_button";
})()
'''


def check_ready_js() -> str:
    """JS to check if Gemini's input is ready."""
    return '''
(function() {
    var input = document.querySelector('.ql-editor[contenteditable="true"]')
             || document.querySelector('div[contenteditable="true"][role="textbox"]');
    return input ? "ready" : "loading";
})()
'''
