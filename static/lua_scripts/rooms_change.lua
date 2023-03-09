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

function Handler:add(input)
    local session_name = input.name or (redis.call("EXISTS", self:session_key(input.id)) == 1 and
        redis.call("GET", self:session_key(input.id))) or "undefined"
    redis.call("SET", self:session_key(input.id), session_name)
    redis.call("SADD", self:session_rooms_key(input.id), input.room)
    redis.call("SET", self:room_key(input.room), input.room)
    redis.call("SADD", self:room_sessions_key(input.room), input.id)
end

function Handler:del(input)
    if redis.call("SREM", self:room_sessions_key(input.room), input.id) == 1 then
        if redis.call("EXISTS", self:room_sessions_key(input.room)) == 0 then
            redis.call("DEL", self:room_key(input.room))
        end
    end
    if redis.call("SREM", self:session_rooms_key(input.id), input.room) == 1 then
        if redis.call("EXISTS", self:session_rooms_key(input.id)) == 0 then
            redis.call("DEL", self:session_key(input.id))
        end
    end
end

function Handler:name_change(input)
    if (input.name and redis.call("EXISTS", self:session_key(input.id)) == 1) then
        redis.call("SET", self:session_key(input.id), input.name)
    end
end

function Handler:handle()
    local input = json.decode(ARGV[1]);
    local output = {
        status = 0,
        msg = ""
    }
    if (input.type == "Add") then
        self:add(input)
    elseif (input.type == "Del") then
        self:del(input)
    elseif (input.type == "NameChange") then
        self:name_change(input)
    else
        output.status = 1;
        output.msg = string.format("wrong type: %q", input.type);
    end
    return json.encode(output)
end

return Handler:new():handle()
