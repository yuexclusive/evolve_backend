--[[
Handler Object, for change the room and session
--]]
local Handler = {}

function Handler:new()
    local res = {}
    setmetatable(res, {
        __index = self
    })
    return res
end

function Handler:session_key(id)
    return "ws_session_" .. id
end

function Handler:session_rooms_key(id)
    return "ws_session_" .. id .. "_rooms"
end

function Handler:room_key(id)
    return "ws_room_" .. id
end

function Handler:room_sessions_key(id)
    return "ws_room_" .. id .. "_sessions"
end

function Handler:get_by_session_id(id)
    local room_ids = redis.call("SMEMBERS", self:session_rooms_key(id))
    for _, r_id in pairs(room_ids) do
        local session_ids = redis.call("SMEMBERS", self:room_sessions_key(r_id))
        for _, s_id in pairs(session_ids) do
            self[r_id]       = self[r_id] or {}
            self[r_id][s_id] = redis.call("GET", self:session_key(s_id))
        end
    end
end

function Handler:get_by_room_id(r_id)
    local session_ids = redis.call("SMEMBERS", self:room_sessions_key(r_id))
    for _, s_id in pairs(session_ids) do
        self[r_id]       = self[r_id] or {}
        self[r_id][s_id] = redis.call("GET", self:session_key(s_id))
    end
end

function Handler:handle()
    local input = json.decode(ARGV[1]);
    if (input.type == "get_by_session_id") then
        self:get_by_session_id(input.id)
    elseif (input.type == "get_by_room_id") then
        self:get_by_room_id(input.id)
    end
    return json.encode(self)
end

return Handler:new():handle()
