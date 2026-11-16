function compute_ai_commands(state)
    local commands = {}
    local team = state.team or 0
    local difficulty = state.difficulty or 1
    local tick = state.tick or 0
    local resources = state.resources or 0
    local supply_used = state.supply_used or 0
    local supply_cap = state.supply_cap or 0
    local own_units = state.own_units or {}
    local own_buildings = state.own_buildings or {}
    local resource_nodes = state.resource_nodes or {}
    local visible_enemies = state.visible_enemies or {}
    local enemy_base_pos = state.enemy_base_pos or { x = 50.0, y = 50.0 }

    -- Difficulty parameters
    local max_workers = 4
    local decision_interval = 60
    if difficulty == 2 then
        max_workers = 7
        decision_interval = 45
    elseif difficulty >= 3 then
        max_workers = 10
        decision_interval = 30
    end

    -- Throttle AI decisions based on difficulty / tick to save CPU and stagger actions
    if tick % decision_interval ~= (team * 15) % decision_interval then
        return commands
    end

    -- Count workers, soldiers, snipers, barracks, mines, houses
    local workers = {}
    local idle_workers = {}
    local soldiers = {}
    local snipers = {}
    local combat_unit_ids = {}

    for _, u in ipairs(own_units) do
        if u.unit_type == "worker" then
            table.insert(workers, u)
            if u.order == "Idle" or u.order == "idle" then
                table.insert(idle_workers, u)
            end
        elseif u.unit_type == "soldier" then
            table.insert(soldiers, u)
            table.insert(combat_unit_ids, u.id)
        elseif u.unit_type == "sniper" then
            table.insert(snipers, u)
            table.insert(combat_unit_ids, u.id)
        end
    end

    local command_centers = {}
    local barracks_list = {}
    local mines = {}
    local houses = {}
    local damaged_buildings = {}

    for _, b in ipairs(own_buildings) do
        if b.completed then
            if b.building_type == "command_center" then
                table.insert(command_centers, b)
            elseif b.building_type == "barracks" then
                table.insert(barracks_list, b)
            elseif b.building_type == "mine" then
                table.insert(mines, b)
            elseif b.building_type == "house" then
                table.insert(houses, b)
            end
            if b.hp < b.max_hp then
                table.insert(damaged_buildings, b)
            end
        end
    end

    -- 1. Repair Logic (Medium & Hard actively repair damaged buildings)
    if difficulty >= 2 and #damaged_buildings > 0 and #idle_workers > 0 then
        local w = idle_workers[1]
        table.remove(idle_workers, 1)
        local target_b = damaged_buildings[1]
        table.insert(commands, {
            action = "repair",
            worker_id = w.id,
            building_id = target_b.id
        })
    end

    -- 2. Worker Training
    if #workers < max_workers and resources >= 50 and supply_used < supply_cap and #command_centers > 0 then
        local cc = command_centers[1]
        if (cc.queue_len or 0) == 0 then
            table.insert(commands, {
                action = "train",
                building_id = cc.id,
                unit_type = "worker"
            })
            resources = resources - 50
        end
    end

    -- 3. Supply / House Building
    if supply_used >= supply_cap - 2 and resources >= 100 and #idle_workers > 0 then
        local w = idle_workers[1]
        table.remove(idle_workers, 1)
        -- Build house near command center
        local cc_pos = command_centers[1] and { x = command_centers[1].x, y = command_centers[1].y } or { x = 20.0, y = 20.0 }
        table.insert(commands, {
            action = "build",
            worker_id = w.id,
            building_type = "house",
            x = cc_pos.x + 4.0,
            y = cc_pos.y + 4.0
        })
        resources = resources - 100
    end

    -- 4. Mine Expansion (Easy = 1-2 mines, Medium = 3, Hard = all available)
    local target_mine_count = 1
    if difficulty == 2 then
        target_mine_count = 3
    elseif difficulty >= 3 then
        target_mine_count = 10 -- all
    end

    if #mines < target_mine_count and resources >= 75 and #idle_workers > 0 then
        -- Find unmined resource node
        for _, node in ipairs(resource_nodes) do
            if not node.has_mine and (node.side_hint == team or difficulty >= 3) then
                local w = idle_workers[1]
                table.remove(idle_workers, 1)
                table.insert(commands, {
                    action = "build",
                    worker_id = w.id,
                    building_type = "mine",
                    x = node.x,
                    y = node.y,
                    node_id = node.id
                })
                resources = resources - 75
                break
            end
        end
    end

    -- 5. Barracks Building (Medium & Hard build barracks)
    if difficulty >= 2 and #barracks_list == 0 and resources >= 175 and #idle_workers > 0 then
        local w = idle_workers[1]
        table.remove(idle_workers, 1)
        local cc_pos = command_centers[1] and { x = command_centers[1].x, y = command_centers[1].y } or { x = 20.0, y = 20.0 }
        table.insert(commands, {
            action = "build",
            worker_id = w.id,
            building_type = "barracks",
            x = cc_pos.x - 5.0,
            y = cc_pos.y + 3.0
        })
        resources = resources - 175
    end

    -- 6. Harvesting Logic for Idle Workers near mines
    for _, w in ipairs(idle_workers) do
        if w.order == "Idle" or w.order == "idle" then
            -- Find closest mine with resources
            local best_mine = nil
            local min_dist = 999999.0
            for _, m in ipairs(mines) do
                local dx = m.x - w.x
                local dy = m.y - w.y
                local dist = dx * dx + dy * dy
                if dist < min_dist then
                    min_dist = dist
                    best_mine = m
                end
            end
            if best_mine then
                table.insert(commands, {
                    action = "harvest",
                    worker_id = w.id,
                    target_id = best_mine.id
                })
            end
        end
    end

    -- 7. Combat Unit Training from Barracks
    if #barracks_list > 0 and resources >= 100 and supply_used < supply_cap then
        for _, b in ipairs(barracks_list) do
            if (b.queue_len or 0) == 0 then
                local unit_type = "soldier"
                -- Difficulty squad composition: Easy = mostly soldiers, Medium = mixed, Hard = combined arms (soldiers + snipers)
                if difficulty >= 3 and #snipers < #soldiers * 0.5 and resources >= 200 then
                    unit_type = "sniper"
                elseif difficulty == 2 and math.random() < 0.3 and resources >= 200 then
                    unit_type = "sniper"
                end

                local cost = (unit_type == "sniper") and 200 or 100
                if resources >= cost then
                    table.insert(commands, {
                        action = "train",
                        building_id = b.id,
                        unit_type = unit_type
                    })
                    resources = resources - cost
                end
            end
        end
    end

    -- 8. Combat & Squad Commands (Attack / Attack-Move)
    if #combat_unit_ids >= (difficulty == 1 and 4 or 2) then
        if #visible_enemies > 0 then
            -- Hard/Medium focus fire or attack target
            local target = visible_enemies[1]
            if difficulty >= 2 then
                table.insert(commands, {
                    action = "attack_target",
                    unit_ids = combat_unit_ids,
                    target_id = target.id
                })
            else
                table.insert(commands, {
                    action = "attack_move",
                    unit_ids = combat_unit_ids,
                    x = target.x,
                    y = target.y
                })
            end
        elseif difficulty >= 2 and tick % 120 == 0 then
            -- Attack enemy base / advance
            table.insert(commands, {
                action = "attack_move",
                unit_ids = combat_unit_ids,
                x = enemy_base_pos.x,
                y = enemy_base_pos.y
            })
        end
    end

    return commands
end
