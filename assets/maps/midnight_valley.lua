return {
    id = "midnight_valley",
    name = "Midnight Valley",
    description = "A classic balanced valley map with dual plateaus and central choke points.",
    width = 96.0,
    height = 96.0,
    team_spawns = { {18.0, 18.0}, {78.0, 78.0} },
    resource_nodes = {
        -- Side 0 (4 nodes)
        { position = { 22.0, 15.0 }, amount = 2500, side_hint = 0 },
        { position = { 15.0, 22.0 }, amount = 2500, side_hint = 0 },
        { position = { 26.0, 20.0 }, amount = 2500, side_hint = 0 },
        { position = { 20.0, 26.0 }, amount = 2500, side_hint = 0 },
        -- Side 1 (4 nodes)
        { position = { 74.0, 81.0 }, amount = 2500, side_hint = 1 },
        { position = { 81.0, 74.0 }, amount = 2500, side_hint = 1 },
        { position = { 70.0, 76.0 }, amount = 2500, side_hint = 1 },
        { position = { 76.0, 70.0 }, amount = 2500, side_hint = 1 }
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
