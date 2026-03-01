# Noise Bank

Place game audio WAV files here for noise augmentation during training.

Files should be:
- 16kHz mono WAV (preferred, but other sample rates are auto-resampled)
- Any duration (will be looped/cropped to match training samples)

Suggested categories:
- Combat sounds (weapon impacts, explosions, projectiles)
- Ambient environment (forest, water, wind, rain)
- UI sounds (selection clicks, building construction)
- Music (game soundtrack or similar genre)

During training, these clips are randomly mixed with keyword audio
at SNR ratios between 0dB (very noisy) and 20dB (mild background).
