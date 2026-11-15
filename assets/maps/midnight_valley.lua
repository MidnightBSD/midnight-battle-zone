return {
    id = "midnight_valley",
    name = "Midnight Valley",
    description = "A classic balanced valley with breakable central rock barriers and mine-shielding outcrops.",
    width = 96.0,
    height = 96.0,
    team_spawns = { {18.0, 18.0}, {78.0, 78.0} },
    resource_nodes = {
        -- Side 0 (4 nodes, spread from the home corner toward the middle)
        { position = { 18.0, 18.0 }, amount = 2500, side_hint = 0 },
        { position = { 34.0, 24.0 }, amount = 2500, side_hint = 0 },
        { position = { 22.0, 40.0 }, amount = 2500, side_hint = 0 },
        { position = { 38.0, 42.0 }, amount = 2500, side_hint = 0 },
        -- Side 1 (180-degree mirror of side 0)
        { position = { 78.0, 78.0 }, amount = 2500, side_hint = 1 },
        { position = { 62.0, 72.0 }, amount = 2500, side_hint = 1 },
        { position = { 74.0, 56.0 }, amount = 2500, side_hint = 1 },
        { position = { 58.0, 54.0 }, amount = 2500, side_hint = 1 }
    },
    obstacles = {
        -- Large breakable central rock barrier between the two halves
        { position = { 48.0, 48.0 }, radius = 8.0, height = 11.0, max_hp = 2400.0 },
        { position = { 41.0, 55.0 }, radius = 4.8, height = 7.5, max_hp = 1300.0 },
        { position = { 55.0, 41.0 }, radius = 4.8, height = 7.5, max_hp = 1300.0 },
        -- Flanking rock structures partially shielding mid and side mines
        { position = { 35.0, 60.0 }, radius = 4.0, height = 6.0, max_hp = 900.0 },
        { position = { 61.0, 36.0 }, radius = 4.0, height = 6.0, max_hp = 900.0 },
        { position = { 28.0, 44.0 }, radius = 3.2, height = 5.0, max_hp = 700.0 },
        { position = { 68.0, 52.0 }, radius = 3.2, height = 5.0, max_hp = 700.0 },
        { position = { 40.0, 28.0 }, radius = 3.2, height = 5.0, max_hp = 700.0 },
        { position = { 56.0, 68.0 }, radius = 3.2, height = 5.0, max_hp = 700.0 }
    }
}
