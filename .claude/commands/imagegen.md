# ChatGPT Sprite Generation Skill

Generate game sprites via ChatGPT browser automation.

## Lessons Learned

### ChatGPT Input via JavaScript
- Use `document.querySelector('#prompt-textarea')` — it's a contenteditable div
- Set content with `.innerHTML = lines.map(line => '<p>' + (line || '<br>') + '</p>').join('')`
- MUST dispatch `new Event('input', { bubbles: true })` for React to pick it up
- **Best method**: Click send via JS in the same call using `setTimeout(() => { const btn = document.querySelector('button[data-testid="send-button"]') || document.querySelector('button[aria-label="Send prompt"]'); if (btn) btn.click(); }, 500);`
- Alternatively use `find` tool to locate "Send prompt" button by ref, then click via ref
- Coordinate clicks on the send button are unreliable — avoid them

### ChatGPT Model Selection
- **Never use Pro mode** for image gen — it adds extended thinking (60s+) and often rewrites/reinterprets prompts
- Switch to standard ChatGPT 5.2 by clicking the "Pro" dropdown and deselecting
- Standard mode generates images in ~15-20s

### Prompt Issues
- ChatGPT aggressively reinterprets prompts. "Redwall-meets-Into-the-Breach art style" caused it to generate an otter instead of a cat
- **Keep prompts short and direct.** Don't combine style + character in a long paragraph
- Emphasize the SUBJECT (animal type, pose) at the top, style at the bottom
- Use explicit corrections: "I need a CAT, not an otter"
- ChatGPT will auto-generate additional portraits unprompted — send "STOP. Only generate what I ask" if needed

### Download Workflow
1. **Best method**: Click on the generated image to open the viewer modal
2. In the viewer, click the "Save" button (top-right corner, ~1393,22)
3. File appears in ~/Downloads as `ChatGPT Image {date}.png`
4. Verify with `Read` tool before processing
5. **Alternative**: Use JavaScript `document.querySelectorAll('button[aria-label="Download this image"]')` — but these only appear on hover and are unreliable
6. Download buttons from `aria-label` work in multi-image chats; for single images, the viewer Save is more reliable

### Processing Pipeline
```bash
# Copy raw image
cp "~/Downloads/ChatGPT Image {date}.png" tools/asset_pipeline/raw/units/{name}_raw.png

# Process (removes bg, crops, resizes to 128x128 canvas)
python3 tools/asset_pipeline/scripts/process_sprite.py \
  tools/asset_pipeline/raw/units/{name}_raw.png \
  assets/sprites/units/{name}_idle.png \
  --width 128 --height 128
```

### Neutral Gray Requirement
- Game applies team color as multiply tint — sprites MUST be neutral gray (#B0B0B0 to #D0D0D0)
- ChatGPT often ignores this. Explicitly say: "body/fur must be NEUTRAL LIGHT GRAY, NOT brown, NOT colored"
- Only accessories (eyes, tools, equipment) get color

### Efficient Multi-Sprite Generation
- Use ONE ChatGPT chat per sprite to avoid context pollution
- Start each new chat with `New chat` button
- Keep on standard mode (not Pro)
- Add "Generate ONLY this image. Do not create additional characters." to each prompt
- Optimal prompt structure:
  ```
  Generate a 128x128 isometric sprite for a 2D RTS game.

  Subject: [ANIMAL TYPE] — [brief description with pose]

  Requirements:
  - Isometric view (~30 degrees), facing south-east
  - Transparent PNG background
  - Body/fur in neutral gray (#B0B0B0-#D0D0D0) — team color applied in-engine
  - Clean vector art, flat colors, bold dark outlines (2-3px)
  - Into the Breach / Northgard aesthetic
  - No gradients, 2-3 value steps per hue

  Generate ONLY this one image. Do not add extra characters.
  ```

## Path Mapping
| Unit | File Path |
|------|-----------|
| Pawdler | sprites/units/pawdler_idle.png |
| Nuisance | sprites/units/nuisance_idle.png |
| Chonk | sprites/units/chonk_idle.png |
| FlyingFox | sprites/units/flying_fox_idle.png |
| Hisser | sprites/units/hisser_idle.png |
| Yowler | sprites/units/yowler_idle.png |
| Mouser | sprites/units/mouser_idle.png |
| Catnapper | sprites/units/catnapper_idle.png |
| FerretSapper | sprites/units/ferret_sapper_idle.png |
| MechCommander | sprites/units/mech_commander_idle.png |
