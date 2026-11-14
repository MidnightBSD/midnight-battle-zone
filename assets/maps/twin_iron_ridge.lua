return {
    id = "twin_iron_ridge",
    name = "Twin Iron Ridge",
    description = "A rugged terrain split by a central mountain ridge with two narrow bridge passages.",
    width = 84.0,
    height = 84.0,
    team_spawns = { {16.0, 42.0}, {68.0, 42.0} },
    resource_nodes = {
        -- Side 0 (3 nodes)
        { position = { 20.0, 36.0 }, amount = 2500, side_hint = 0 },
        { position = { 20.0, 48.0 }, amount = 2500, side_hint = 0 },
        { position = { 24.0, 42.0 }, amount = 2500, side_hint = 0 },
        -- Side 1 (3 nodes)
        { position = { 64.0, 36.0 }, amount = 2500, side_hint = 1 },
        { position = { 64.0, 48.0 }, amount = 2500, side_hint = 1 },
        { position = { 60.0, 42.0 }, amount = 2500, side_hint = 1 }
    },
    obstacles = {
        -- Central ridge blocking direct path except top and bottom bridges
        { position = { 42.0, 25.0 }, radius = 8.0, height = 12.0 },
        { position = { 42.0, 59.0 }, radius = 8.0, height = 12.0 },
        { position = { 42.0, 42.0 }, radius = 5.0, height = 10.0 }
    }
}
