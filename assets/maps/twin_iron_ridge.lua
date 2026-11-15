return {
    id = "twin_iron_ridge",
    name = "Twin Iron Ridge",
    description = "A rugged pass divided by a massive breakable central rock ridge and fortified mine pockets.",
    width = 84.0,
    height = 84.0,
    team_spawns = { {16.0, 42.0}, {68.0, 42.0} },
    resource_nodes = {
        -- Side 0 (3 nodes, north, center, and south of the ridge)
        { position = { 16.0, 16.0 }, amount = 2500, side_hint = 0 },
        { position = { 28.0, 42.0 }, amount = 2500, side_hint = 0 },
        { position = { 16.0, 68.0 }, amount = 2500, side_hint = 0 },
        -- Side 1 (180-degree mirror of side 0)
        { position = { 68.0, 68.0 }, amount = 2500, side_hint = 1 },
        { position = { 56.0, 42.0 }, amount = 2500, side_hint = 1 },
        { position = { 68.0, 16.0 }, amount = 2500, side_hint = 1 }
    },
    obstacles = {
        -- Large breakable central rock massif between the two halves
        { position = { 42.0, 42.0 }, radius = 8.2, height = 12.0, max_hp = 1300.0 },
        { position = { 42.0, 26.0 }, radius = 6.2, height = 10.0, max_hp = 900.0 },
        { position = { 42.0, 58.0 }, radius = 6.2, height = 10.0, max_hp = 900.0 },
        -- Mine-shielding breakable rocks on each side
        { position = { 23.0, 21.0 }, radius = 3.2, height = 5.5, max_hp = 360.0 },
        { position = { 61.0, 63.0 }, radius = 3.2, height = 5.5, max_hp = 360.0 },
        { position = { 23.0, 63.0 }, radius = 3.2, height = 5.5, max_hp = 360.0 },
        { position = { 61.0, 21.0 }, radius = 3.2, height = 5.5, max_hp = 360.0 }
    }
}
