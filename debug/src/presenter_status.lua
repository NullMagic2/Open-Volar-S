-- Read-only presentation telemetry; no media files or system profiles are changed.
local mp = require 'mp'
local utils = require 'mp.utils'
local directory = utils.split_path(debug.getinfo(1, 'S').source:sub(2))
local output = directory .. '/presenter-state.json'
local function report()
    local state = {
        video = mp.get_property_native('video-out-params'),
        window = mp.get_property_native('osd-dimensions'),
        displays = mp.get_property_native('display-names'),
        display_fps = mp.get_property_native('display-fps'),
        video_fps = mp.get_property_native('estimated-vf-fps'),
        hardware_decoder = mp.get_property_native('hwdec-current'),
        video_output = mp.get_property_native('current-vo'),
        time_position = mp.get_property_native('time-pos'),
    }
    local f = io.open(output, 'w')
    if f then f:write(utils.format_json(state)); f:close() end
end
mp.add_periodic_timer(0.5, report)
mp.register_event('shutdown', report)
