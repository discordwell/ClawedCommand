-- @name: activate_loaf
-- @events: on_tick
-- @interval: 10

-- Gen 16: Activate LoafMode on stationary attacking Chonks.
-- LoafMode (slot 1, 0-indexed) = Toggle, 0 GPU, 10 tick CD.
-- Without persistent state (_G is read-only in sandbox), just
-- toggle on when Chonk is stationary+attacking. The toggle
-- will auto-deactivate if the Chonk starts moving.

local chonks = ctx:my_units("Chonk")
if not chonks then return end

for _, c in ipairs(chonks) do
    if not c.is_dead and not c.moving and c.attacking then
        -- Toggle LoafMode on (slot 1 = index 1, 0-based)
        ctx:ability(c.id, 1, "self")
    end
end
