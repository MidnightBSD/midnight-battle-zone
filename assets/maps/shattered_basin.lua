return {
    id = "shattered_basin",
    name = "Shattered Basin",
    description = "An expansive resource-rich arena with deposits spread across each half and tactical rock clusters.",
    width = 112.0,
    height = 112.0,
    team_spawns = { {20.0, 20.0}, {92.0, 92.0} },
    resource_nodes = {
        -- Side 0 (6 nodes, distributed across the south-west half)
        { position = { 16.0, 16.0 }, amount = 3000, side_hint = 0 },
        { position = { 34.0, 18.0 }, amount = 3000, side_hint = 0 },
        { position = { 18.0, 34.0 }, amount = 3000, side_hint = 0 },
        { position = { 44.0, 30.0 }, amount = 3000, side_hint = 0 },
        { position = { 30.0, 44.0 }, amount = 3000, side_hint = 0 },
        { position = { 45.0, 45.0 }, amount = 3000, side_hint = 0 },
        -- Side 1 (180-degree mirror of side 0)
        { position = { 96.0, 96.0 }, amount = 3000, side_hint = 1 },
        { position = { 78.0, 94.0 }, amount = 3000, side_hint = 1 },
        { position = { 94.0, 78.0 }, amount = 3000, side_hint = 1 },
        { position = { 68.0, 82.0 }, amount = 3000, side_hint = 1 },
        { position = { 82.0, 68.0 }, amount = 3000, side_hint = 1 },
        { position = { 67.0, 67.0 }, amount = 3000, side_hint = 1 }
    },
    obstacles = {
        -- Tactical rock clusters throughout the basin
        { position = { 56.0, 56.0 }, radius = 7.0, height = 9.0 },
        { position = { 40.0, 72.0 }, radius = 5.0, height = 7.0 },
        { position = { 72.0, 40.0 }, radius = 5.0, height = 7.0 },
        { position = { 30.0, 90.0 }, radius = 4.5, height = 6.0 },
        { position = { 82.0, 22.0 }, radius = 4.5, height = 6.0 },
        { position = { 56.0, 25.0 }, radius = 4.0, height = 5.0 },
        { position = { 25.0, 56.0 }, radius = 4.0, height = 5.0 }
    }
}
