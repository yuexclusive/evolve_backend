-- json = require "json"
-- dofile("json.lua")
local ChatServerForHub = {}

function ChatServerForHub:new()
    local data_str = redis.call("GET", KEYS[1])
    local t = data_str and json.decode(data_str) or { sessions = {}, rooms = {}, session_room_map = {} }
    setmetatable(t, { __index = self })
    return t
end

function ChatServerForHub:add(input)
    self.sessions[input.id]                     = input.name or self.sessions[input.id] or "undefined"
    self.rooms[input.room]                      = self.rooms[input.room] or {}
    self.rooms[input.room][input.id]            = true
    self.session_room_map[input.id]             = self.session_room_map[input.id] or {}
    self.session_room_map[input.id][input.room] = true
end

function ChatServerForHub:del(input)
    local function get_len(arr)
        local res = 0
        for k, v in pairs(arr) do
            res = res + 1
        end
        return res
    end

    if self.rooms[input.room] then
        self.rooms[input.room][input.id] = nil
    end
    if (get_len(self.rooms[input.room]) == 0) then
        self.rooms[input.room] = nil
    end
    if self.session_room_map[input.id] then
        self.session_room_map[input.id][input.room] = nil
    end
    if (get_len(self.session_room_map[input.id]) == 0) then
        self.session_room_map[input.id] = nil
        self.sessions[input.id] = nil
    end
end

function ChatServerForHub:name_change(input)
    if (self.sessions[input.id] and input.name) then
        self.sessions[input.id] = input.name
    end
end

function ChatServerForHub:handle()
    local input = json.decode(ARGV[1]);
    local output = { status = 0, msg = "" }
    if (input.type == "Add") then
        self:add(input)
    elseif (input.type == "Del") then
        self:del(input)
    elseif (input.type == "NameChange") then
        self:input(input)
    else
        output.status = 1;
        output.msg = string.format("wrong type: %q", input.type);
    end
    redis.call("SET", KEYS[1], json.encode(self))
    return json.encode(output)
end

return ChatServerForHub:new():handle()
