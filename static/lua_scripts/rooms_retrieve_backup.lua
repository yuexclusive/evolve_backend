--[[
Handler Object, for change the room and session
--]]
local Handler = {}

function Handler:new()
    local res = {
        sessions = {},
        rooms = {},
        session_room_map = {}
    }
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
            self.sessions[s_id]               = redis.call("GET", self:session_key(s_id))
            self.rooms[r_id]                  = self.rooms[r_id] or {}
            self.rooms[r_id][s_id]            = true
            self.session_room_map[s_id]       = self.session_room_map[s_id] or {}
            self.session_room_map[s_id][r_id] = true
        end
    end
end

function Handler:get_by_room_id(r_id)
    local session_ids = redis.call("SMEMBERS", self:room_sessions_key(r_id))
    for _, s_id in pairs(session_ids) do
        local s_name                      = redis.call("GET", self:session_key(s_id))
        self.sessions[s_id]               = s_name
        self.rooms[r_id]                  = self.rooms[r_id] or {}
        self.rooms[r_id][s_id]            = true
        self.session_room_map[s_id]       = self.session_room_map[s_id] or {}
        self.session_room_map[s_id][r_id] = true
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
