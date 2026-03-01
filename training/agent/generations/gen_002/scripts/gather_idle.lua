-- @name: gather_idle
-- @events: on_unit_idle
-- @interval: 15

-- Assign idle Pawdlers to nearest resource deposit.
-- The FSM's `assign_idle_workers` behavior runs, but there can be gaps
-- (especially after workers finish building something). This catches
-- idle workers that the FSM missed.

local idle_workers = ctx:idle_units("Pawdler")
if #idle_workers == 0 then return end

-- For each idle worker, find nearest deposit and send them
for _, w in ipairs(idle_workers) do
  if w.is_dead then goto skip end
  local deposit = ctx:nearest_deposit(w.x, w.y)
  if deposit then
    ctx:gather({w.id}, deposit.id)
  end
  ::skip::
end
