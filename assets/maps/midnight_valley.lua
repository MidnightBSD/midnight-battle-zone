return {
    id = "midnight_valley",
    name = "Midnight Valley",
    description = "A classic balanced valley with scattered deposits, dual plateaus, and central choke points.",
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
        -- Symmetric central and flanking rock clusters
        { position = { 48.0, 48.0 }, radius = 6.0, height = 8.0 },
        { position = { 35.0, 60.0 }, radius = 4.0, height = 6.0 },
        { position = { 61.0, 36.0 }, radius = 4.0, height = 6.0 },
        { position = { 25.0, 50.0 }, radius = 3.5, height = 5.0 },
        { position = { 71.0, 46.0 }, radius = 3.5, height = 5.0 }
    }
}
