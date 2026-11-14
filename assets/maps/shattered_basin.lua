return {
    id = "shattered_basin",
    name = "Shattered Basin",
    description = "An expansive resource-rich arena with scattered tactical rock clusters and abundant deposits.",
    width = 112.0,
    height = 112.0,
    team_spawns = { {20.0, 20.0}, {92.0, 92.0} },
    resource_nodes = {
        -- Side 0 (6 nodes)
        { position = { 25.0, 18.0 }, amount = 3000, side_hint = 0 },
        { position = { 18.0, 25.0 }, amount = 3000, side_hint = 0 },
        { position = { 30.0, 25.0 }, amount = 3000, side_hint = 0 },
        { position = { 25.0, 30.0 }, amount = 3000, side_hint = 0 },
        { position = { 35.0, 20.0 }, amount = 3000, side_hint = 0 },
        { position = { 20.0, 35.0 }, amount = 3000, side_hint = 0 },
        -- Side 1 (6 nodes)
        { position = { 87.0, 94.0 }, amount = 3000, side_hint = 1 },
        { position = { 94.0, 87.0 }, amount = 3000, side_hint = 1 },
        { position = { 82.0, 87.0 }, amount = 3000, side_hint = 1 },
        { position = { 87.0, 82.0 }, amount = 3000, side_hint = 1 },
        { position = { 77.0, 92.0 }, amount = 3000, side_hint = 1 },
        { position = { 92.0, 77.0 }, amount = 3000, side_hint = 1 }
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
