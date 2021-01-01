-- define window size
if os.getenv("DISPLAY") == ":0.0" then
    width = 300; height = 300
else
    width = 200; height = 200
end

-- define background color
-- background = {red = 0.30, green = 0.10, blue = 0}
background = RED