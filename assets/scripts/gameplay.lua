function calculate_damage(base_damage, target_armor, armor_piercing)
    return math.max(1.0, base_damage - math.max(0.0, target_armor - armor_piercing))
end

function get_economy_config()
    return {
        starting_resources = 300,
        harvest_amount_per_trip = 15,
        harvest_ticks = 25,
        repair_hp_per_tick = 3.5,
        repair_cost_per_tick = 1
    }
end

function check_victory_condition(team0_buildings, team1_buildings)
    if team0_buildings == 0 and team1_buildings == 0 then
        return 1 -- or draw, but team 1 loses last
    elseif team0_buildings == 0 then
        return 1
    elseif team1_buildings == 0 then
        return 0
    else
        return -1
    end
end
