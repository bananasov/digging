-- WebSocket broadcaster for SimplifyDig.

---@class bananasov.Broadcaster.WebSocket : SimplifyDig.Broadcaster
---@field websocket_url string The WebSocket URL to connect to.
---@field ws ccTweaked.http.Websocket? The WebSocket connection handle.
local WebSocketBroadcaster = {
    ready = false,
    websocket_url = "ws://127.0.0.1:3000/ws",
    ws = nil,
}

--- Sets up anything the broadcaster needs.
---@param parsed_args argparse-parsed Arguments passed to the program.
---@return boolean success Whether the setup was successful.
---@return string? error An error message if the setup failed.
function WebSocketBroadcaster.setup(parsed_args)
    -- Check if WebSockets are enabled
    if not http.websocket then
        return false, "WebSockets are not enabled. Enable http_websocket_enabled in ComputerCraft config."
    end

    -- Allow custom WebSocket URL via command line argument
    if parsed_args.options.websocket_url then
        WebSocketBroadcaster.websocket_url = parsed_args.options.websocket_url
    end

    -- Attempt to connect to the WebSocket server
    local success, ws_or_error = pcall(http.websocket, WebSocketBroadcaster.websocket_url)

    if not success or not ws_or_error then
        return false, "Failed to connect to WebSocket server: " .. tostring(ws_or_error or "unknown error")
    end

    WebSocketBroadcaster.ws = ws_or_error
    WebSocketBroadcaster.ready = true
    return true
end

--- Sends a raw message.
---@param message SimplifyDig.Broadcaster.Message The message to send.
function WebSocketBroadcaster.raw(message)
    if not WebSocketBroadcaster.ready then
        error("Broadcaster not ready. Call setup() first.")
    end

    if not WebSocketBroadcaster.ws then
        error("WebSocket connection not established.")
    end

    local json_message = textutils.serializeJSON(message)
    WebSocketBroadcaster.ws.send(json_message, false)
end

--- Parses a value to its correct type (number or string)
---@param value any The value to parse
---@return any The parsed value
local function parse_value(value)
    if type(value) == "string" then
        local num = tonumber(value)
        if num then
            return num
        end
    end
    return value
end

--- Sends the init message.
--- This message contains information about the dig (size, quarrying, etc) and
--- is sent once at the start of the dig.
---@param program_arguments argparse-parsed The arguments passed to the program.
function WebSocketBroadcaster.init(program_arguments)
    WebSocketBroadcaster.raw {
        type = "init",
        data = {
            program_arguments = program_arguments,
            turtle_id = os.getComputerID(),
        },
    }
end

--- Broadcast a keepalive message.
function WebSocketBroadcaster.keepalive()
    WebSocketBroadcaster.raw {
        type = "keepalive",
        ---@diagnostic disable-next-line: assign-type-mismatch
        data = nil,
    }
end

--- Update the state of the turtle.
---@param state SimplifyDig.Broadcaster.States The current state of the turtle.
function WebSocketBroadcaster.state(state)
    WebSocketBroadcaster.raw {
        type = "state",
        data = {
            state = state,
        },
    }
end

--- Send a basic status update message.
---@param pos DTR.State.Position The position to send.
---@param facing DTR.State.Facing The facing to send.
---@param fuel integer|"unlimited" The fuel level to send.
function WebSocketBroadcaster.status(pos, facing, fuel)
    WebSocketBroadcaster.raw {
        type = "status",
        data = {
            pos = pos,
            facing = facing,
            fuel = fuel,
        },
    }
end

--- Sends a completion status message.
---@param completion_percent number The percentage of the dig that is complete, from 0 to 1.
function WebSocketBroadcaster.completion(completion_percent)
    WebSocketBroadcaster.raw {
        type = "completion",
        data = {
            completion_percent = completion_percent,
        },
    }
end

--- Send a message that the dig is complete.
function WebSocketBroadcaster.complete()
    WebSocketBroadcaster.raw {
        type = "complete",
        ---@diagnostic disable-next-line: assign-type-mismatch
        data = nil,
    }

    -- Close the WebSocket connection after sending completion message
    if WebSocketBroadcaster.ws then
        WebSocketBroadcaster.ws.close()
        WebSocketBroadcaster.ws = nil
        WebSocketBroadcaster.ready = false
    end
end

--- Sends a message that the turtle is stuck.
---@param reason string The reason the turtle is stuck.
---@param pos DTR.State.Position The position to send.
---@param facing DTR.State.Facing The facing to send.
---@param fuel integer|"unlimited" The fuel level to send.
function WebSocketBroadcaster.panic(reason, pos, facing, fuel)
    WebSocketBroadcaster.raw {
        type = "panic",
        data = {
            reason = reason,
            pos = pos,
            facing = facing,
            fuel = fuel,
        },
    }
end

--- Send a message stating the turtle has errored.
---@param message string The error message to send.
---@param pos DTR.State.Position The position to send.
---@param facing DTR.State.Facing The facing to send.
---@param fuel integer|"unlimited" The fuel level to send.
function WebSocketBroadcaster.error(message, pos, facing, fuel)
    WebSocketBroadcaster.raw {
        type = "error",
        data = {
            message = message,
            pos = pos,
            facing = facing,
            fuel = fuel,
        },
    }
end

return WebSocketBroadcaster
