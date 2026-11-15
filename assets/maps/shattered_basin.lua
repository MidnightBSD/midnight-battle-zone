return {
    id = "shattered_basin",
    name = "Shattered Basin",
    description = "An expansive arena with a colossal central rock citadel and breakable mine barricades.",
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
        -- Large breakable central rock citadel between the two halves
        { position = { 56.0, 56.0 }, radius = 9.0, height = 12.0, max_hp = 2800.0 },
        { position = { 47.0, 65.0 }, radius = 5.2, height = 8.0, max_hp = 1500.0 },
        { position = { 65.0, 47.0 }, radius = 5.2, height = 8.0, max_hp = 1500.0 },
        -- Flanking and mine-protecting rock clusters throughout the basin
        { position = { 40.0, 72.0 }, radius = 4.8, height = 7.0, max_hp = 1100.0 },
        { position = { 72.0, 40.0 }, radius = 4.8, height = 7.0, max_hp = 1100.0 },
        { position = { 30.0, 90.0 }, radius = 4.2, height = 6.0, max_hp = 900.0 },
        { position = { 82.0, 22.0 }, radius = 4.2, height = 6.0, max_hp = 900.0 },
        { position = { 50.0, 34.0 }, radius = 3.5, height = 5.5, max_hp = 760.0 },
        { position = { 62.0, 78.0 }, radius = 3.5, height = 5.5, max_hp = 760.0 },
        { position = { 34.0, 50.0 }, radius = 3.5, height = 5.5, max_hp = 760.0 },
        { position = { 78.0, 62.0 }, radius = 3.5, height = 5.5, max_hp = 760.0 }
    }
}
