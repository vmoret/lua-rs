-- define window size
if os.getenv("DISPLAY") == ":0.0" then
    width = 300; height = 300
else
    width = 200; height = 200
end

function f (x, y)
    return (x^2 * math.sin(y)) / (1 - x), "foo bar"
end
