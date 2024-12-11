local schema = require("resty.router.schema")
local context = require("resty.router.context")
local router = require("resty.router.router")
local debug = require("resty.router.debug")

local UUIDs = {
    "c6523d1a-9c37-47ff-bf50-7db176145301",
    "6dece4f7-3ba6-4bf9-91e1-2e35574bb612",
    "2af40526-ba49-4dfe-af3e-fbb748785fdd",
    "92a35b38-bf96-4cc5-8f15-1accdf9da3d3",
    "3b327d3e-af84-49b8-aaef-3c0659af83e6",
}

function uuid_generator()
    return UUIDs[math.random(1, #UUIDs)]
end

describe("change-route test", function()
    local uuid = uuid_generator()

    local s = schema.new()
    s:add_field("http.path", "String")
    local c = context.new(s)
    local r = router.new(s, 1)
    local expr = "http.path ^= r#\"/test\"#"

    lazy_setup(function()
        c:reset()
        r:add_matcher(1, uuid, expr)
    end)

    it("test1", function()
        local _, _, _ = debug.router_get_duration(r)
        local _, _, _ = debug.router_get_counter(r)

        for _i = 1, 500000 do
            -- Change
            r:remove_matcher(uuid)
            r:add_matcher(1, uuid, expr)

            -- Access
            c:reset()
            c:add_value("http.path", "/test/1234")
            assert(r:execute(c), "Failed to execute router")
            local _, _ = c:get_result("http.path")
        end

        local add, remove, execute = debug.router_get_duration(r)
        print("Duration in ms:")
        print("add_matcher: " .. add / 1000000)
        print("remove_matcher: " .. remove / 1000000)
        print("execute: " .. execute / 1000000)
        print("Total in ms: " .. (add + remove + execute) / 1000000)
        print()
        add, remove, execute = debug.router_get_counter(r)
        print("Counter:")
        print("add_matcher: " .. add)
        print("remove_matcher: " .. remove)
        print("execute: " .. execute)
        print()
        print()
    end)

end)