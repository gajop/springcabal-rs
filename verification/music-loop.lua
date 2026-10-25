-- The runner clips the real soundtrack to two seconds in a private game copy.
-- Check the actual engine stream rather than just counting PlaySoundStream calls.
function gadget:GetInfo()
    return {name = 'Music loop regression', layer = 100000, enabled = true}
end
if gadgetHandler:IsSyncedCode() then return false end

local began, previous, loops, finished
function gadget:Initialize()
    began, previous, loops, finished = Spring.GetTimer(), 0, 0, false
end
function gadget:Update()
    if finished then return end
    local played, length = Spring.GetSoundStreamTime()
    if length > 0 then
        assert(math.abs(length - 2) < 0.1, 'the clipped soundtrack did not load')
        if played < previous - 0.5 then
            loops = loops + 1
            Spring.Echo('[MusicLoopTest] completed loop ' .. loops)
        end
        previous = played
    end
    if Spring.DiffTimers(Spring.GetTimer(), began) >= 8.5 then
        finished = true
        assert(loops >= 3, 'music stopped looping: observed ' .. loops .. ' restarts')
        Spring.Echo('SHARED MUSIC LOOP PASSED')
        Spring.SendCommands('quitforce')
    end
end
