use std::collections::HashMap;

use rand::Rng;

use crate::client::{types::Stock, Client};

static SIMULATION_TICK_TIMER_IN_SECONDS: u64 = 30;

static SELL_BELOW_AVERAGE_AFTER_SECONDS: u64 = (23 * 60 + 30) * 60; // 23h30m
static DONT_BUY_BASED_ON_TAG_LEVEL_AFTER_SECONDS: u64 = 23 * 60 * 60; // 23h
static DONT_BUY_ONE_OF_EVERYTHING_AFTER_SECONDS: u64 = 22 * 60 * 60; // 22h

static BEDAZZLE_AFTER_SECONDS: u64 = 10 * 60; // 10m

static PRICE_REDUCTION: f64 = 0.1; // Reduce price of unselled items in 10% steps
static PRICE_INCREASE: f64 = 0.05; // Increase price of sold items in 5% steps

static TAG_LEVEL_INCREASE: usize = 2;
static SIMILAR_TAG_LEVEL_INCREASE: usize = 1;
static TAG_LEVEL_BUY_THRESHOLD: usize = 2;

static HIGH_AVERAGE_PRICE_SELLING_MULTIPLIER: f64 = 2.0; // Start selling at twice the average price
static LOW_AVERAGE_PRICE_SELLING_MULTIPIER: f64 = 1.1; // Sell 10% above average price at most
static AVERAGE_PRICE_BUYING_MULTIPIER: f64 = 1.1; // Buy 10% above average price at most

// Battleplan:

// Loop every 30s

// If an article didn't sell:
// --- Reduce it's price by PRICE_INCREASE
// ------ Up to the average
// --------- Kill this rule at the end, so we can sell our stock.
// --- Reduce it's tag level to 0

// If an article did sell:
// --- Increase it's price by PRICE_REDUCTION
// --- Increase it's tag level by TAG_LEVEL_INCREASE
// --- Increase it's similar tags by SIMILAR_TAG_LEVEL_INCREASE

// If an article has a tag level > 0:
// --- Buy an amount that is equal to the tag level
// ------ Kill this rule at the end, so we can sell our stock.

// Try to keep one of every article in stock
// --- Start selling for 200% or the current price
// --- Kill this rule at the end, so we can sell our stock.

// Bedazzle other users
// --- Old listings with a count of 0 will receive a update to hundred times its price
// --- Create some (100?) new listings with a count of 0 and a weird price (even negatives!)

// What are our limits?
// --- Only buy when the price is below the average + 10%
// --- Store half of any winnings in a piggybank, never touch it

pub async fn play(client: &mut Client) {
    let start = std::time::Instant::now();
    let mut tick_timer;

    let mut rng = rand::thread_rng();

    let mut piggybank = 0.0;
    let mut old_player = client.player.clone();
    let mut old_own_listings = client.get_own_listings();

    println!(
        "Starting game loop, playing every {} seconds.",
        SIMULATION_TICK_TIMER_IN_SECONDS
    );

    loop {
        tick_timer = std::time::Instant::now();

        println!();
        println!("Handling simulation tick.");

        // On any problems, we just go to bed and hope for a better day.

        if !client.fetch_player_self().await {
            println!("Unexpected Player-API result, standing down.");
            continue;
        }
        if !client.fetch_articles().await {
            println!("Unexpected Articles-API result, standing down.");
            continue;
        }
        if !client.fetch_tags().await {
            println!("Unexpected Tags-API result, standing down.");
            continue;
        }
        if !client.fetch_suppliers().await {
            println!("Unexpected Suppliers-API result, standing down.");
            continue;
        }
        if !client.fetch_listings().await {
            println!("Unexpected Listings-API result, standing down.");
            continue;
        }

        // Put half of our winnings in our piggybank
        let earnings = client.player.money - old_player.money;
        piggybank += earnings / 2.0;

        println!(
            "Player money: {} (earned {})",
            client.player.money,
            client.player.money - old_player.money
        );

        println!("Piggybank: {} (added {})", piggybank, earnings / 2.0);

        let own_listings = client.get_own_listings();

        let mut portfolio_item_count = 0;
        let mut portfolio_max_value = 0.0;
        let mut portfolio_min_value = 0.0;
        for listing in own_listings.iter() {
            let total_price = listing.count as f64 * listing.price;

            portfolio_item_count += listing.count;
            portfolio_max_value += total_price;
            portfolio_min_value += match client.article_price_history.get(&listing.article) {
                Some(history) => history.average_price(),
                None => total_price / 2.0,
            };
        }
        println!(
            "Portfolio: {} items, approx. value: {} to {}",
            portfolio_item_count, portfolio_min_value, portfolio_max_value
        );

        // Did we sell anything?
        for listing in &own_listings {
            let old_listing = old_own_listings
                .iter()
                .find(|old_listing| old_listing.id == listing.id);
            if old_listing.is_none() {
                eprintln!("Weird. Didn't find an old listing for {}", listing.id);
                continue; // Ideally this never happens
            }

            // get article tags
            let article_tags_and_similar_tags = client.get_tags_for_article_id(listing.article);

            // get article price history
            let article_price_history = match client.article_price_history.get(&listing.article) {
                Some(history) => history,
                None => {
                    eprintln!(
                        "Weird. Didn't find an article price history for {}",
                        listing.article
                    );
                    continue; // Ideally this never happens
                }
            };
            let article_average_price = article_price_history.average_price();

            let sell_count = if old_listing.unwrap().count > listing.count {
                old_listing.unwrap().count - listing.count
            } else {
                0
            };

            if sell_count == 0 {
                // Article didn't sell at all

                // Reduce tag level to 0
                for (tag, _) in article_tags_and_similar_tags.iter() {
                    match client.tag_trend_levels.get_mut(tag) {
                        Some(tag_trend_level) => {
                            *tag_trend_level = 0;
                        }
                        None => {
                            eprintln!("Weird. Didn't find a tag trend level for {}", tag);
                        }
                    }
                }

                // Reduce price down to the average
                let lowered_price = listing.price * (1.0 - PRICE_REDUCTION);
                let low_average_selling_price =
                    article_average_price * LOW_AVERAGE_PRICE_SELLING_MULTIPIER;
                let new_price = if lowered_price > low_average_selling_price
                    || start.elapsed().as_secs() > SELL_BELOW_AVERAGE_AFTER_SECONDS
                {
                    lowered_price
                } else {
                    low_average_selling_price
                };

                client
                    .update_listing(listing.id, listing.count, new_price)
                    .await;
            } else {
                // Article did sell
                println!("Sold {} articles with id {}.", sell_count, listing.article);

                // Increase tag levels
                for (tag, similar_tags) in article_tags_and_similar_tags.iter() {
                    match client.tag_trend_levels.get_mut(tag) {
                        Some(tag_trend_level) => {
                            *tag_trend_level += TAG_LEVEL_INCREASE;
                        }
                        None => {
                            eprintln!("Weird. Didn't find a tag trend level for {}", tag);
                        }
                    }
                    for similar_tag in similar_tags {
                        match client.tag_trend_levels.get_mut(similar_tag) {
                            Some(tag_trend_level) => {
                                *tag_trend_level += SIMILAR_TAG_LEVEL_INCREASE;
                            }
                            None => {
                                eprintln!("Weird. Didn't find a tag trend level for {}", tag);
                            }
                        }
                    }
                }

                // Increase price
                let new_price = listing.price * (1.0 + PRICE_INCREASE);

                client
                    .update_listing(listing.id, listing.count, new_price)
                    .await;
            }
        }

        // Let's go shopping!
        let mut articles_to_buy = HashMap::new();

        // Buy articles according to trending tags
        if start.elapsed().as_secs() < DONT_BUY_BASED_ON_TAG_LEVEL_AFTER_SECONDS {
            for (trending_tag, level) in client
                .tag_trend_levels
                .iter()
                .filter(|(_, tag_trend_level)| *tag_trend_level > &TAG_LEVEL_BUY_THRESHOLD)
            {
                for article in client
                    .articles
                    .iter()
                    .filter(|article| article.tags.contains(trending_tag))
                {
                    let article_count = articles_to_buy.entry(article.id).or_insert(0);
                    *article_count += level;
                }
            }
        }

        // Buy at least one of every article if we don't have it already
        if start.elapsed().as_secs() < DONT_BUY_ONE_OF_EVERYTHING_AFTER_SECONDS {
            for article in client.articles.iter() {
                // Not in our stock
                if !client
                    .player
                    .stock
                    .iter()
                    .any(|player_stock| player_stock.article_id == article.id)
                {
                    // Not in our listings with a count > 0
                    if !own_listings
                        .iter()
                        .any(|listing| listing.article == article.id && listing.count > 0)
                    {
                        articles_to_buy.entry(article.id).or_insert(1);
                    }
                }
            }
        }

        // Limit our purchasing power
        let mut available_money = client.player.money - piggybank;

        // Try to buy articles_to_buy
        'buy_loop: for (article_id, count) in articles_to_buy.iter() {
            // Find suppliers with stock of this article
            let supplier_and_stocks = client
                .suppliers
                .iter()
                .filter_map(|supplier| {
                    let supplier_stock = supplier
                        .stock
                        .iter()
                        .filter(|supplier_stock| {
                            &supplier_stock.article_id == article_id && supplier_stock.stock > 0
                        })
                        .next();
                    match supplier_stock {
                        Some(supplier_stock) => Some((supplier.id, supplier_stock.clone())),
                        None => None,
                    }
                })
                .collect::<Vec<(usize, Stock)>>();

            let mut count = *count;

            // Buy from suppliers until we have our desired count
            for (supplier_id, stock) in supplier_and_stocks {
                // Check if the price is sane
                let article_price_history =
                    match client.article_price_history.get(&stock.article_id) {
                        Some(history) => history,
                        None => {
                            eprintln!(
                                "Weird. Didn't find an article price history for {}",
                                stock.article_id
                            );
                            continue 'buy_loop; // Ideally this never happens
                        }
                    };
                let article_average_price = article_price_history.average_price();
                if stock.price > article_average_price * AVERAGE_PRICE_BUYING_MULTIPIER {
                    // Too expensive, fuck this guy
                    continue 'buy_loop;
                }

                let mut amount_to_buy = if stock.stock < count {
                    // Not enough in stock, we need more than one supplier
                    stock.stock
                } else {
                    count
                };

                while amount_to_buy > 0
                    && available_money - (stock.price * amount_to_buy as f64) < 0.0
                {
                    // Not enough money to buy this, reduce amount to buy
                    amount_to_buy -= 1;
                }

                if amount_to_buy == 0 {
                    // We didn't have enough money for a single thing, skip this item
                    continue 'buy_loop;
                }

                if client
                    .buy_from_supplier(supplier_id, *article_id, amount_to_buy, stock.price)
                    .await
                {
                    // Buy successful, update our money
                    available_money -= stock.price * amount_to_buy as f64;
                    count -= amount_to_buy;
                }

                if count == 0 {
                    // Bought all we needed from this article
                    break;
                }
            }
        }

        // Make sure our local player data is u2d after we changed our stock
        client.fetch_player_self().await;

        // Bedazzle other users
        if start.elapsed().as_secs() > BEDAZZLE_AFTER_SECONDS {
            // Listings with a count of 0 will receive a price update to hundred times the average
            for listing in own_listings.iter().filter(|listing| listing.count == 0) {
                client
                    .update_listing(listing.id, 0, listing.price * 100.0)
                    .await;
                client.bedazzlement_listings.push(listing.id);
            }
            // Create some new listings with a count of 0 and a negative price
            for stock in client.player.stock.clone() {
                let random_price = rng.gen_range(-1000.0..=0.0);
                if let Some(listing_id) = client
                    .create_listing(stock.article_id, 0, random_price)
                    .await
                {
                    client.bedazzlement_listings.push(listing_id);
                }
            }
        }

        // Make sure our local listing is u2d after we changed it
        client.fetch_listings().await;
        let own_listings = client.get_own_listings();

        // Move whole stock to listings
        for stock in client.player.stock.clone() {
            let listing = own_listings
                .iter()
                .map(|listing| listing.clone())
                .find(|listing| listing.article == stock.article_id);

            if let Some(listing) = listing {
                // Update existing listing
                client
                    .update_listing(listing.id, listing.count + stock.stock, listing.price)
                    .await;
            } else {
                // Create a new listing

                let article_price_history =
                    match client.article_price_history.get(&stock.article_id) {
                        Some(history) => history,
                        None => {
                            eprintln!(
                                "Weird. Didn't find an article price history for {}",
                                stock.article_id
                            );
                            continue; // Ideally this never happens
                        }
                    };
                let article_average_price = article_price_history.average_price();

                client
                    .create_listing(
                        stock.article_id,
                        stock.stock,
                        article_average_price * HIGH_AVERAGE_PRICE_SELLING_MULTIPLIER,
                    )
                    .await;
            }
        }

        // Make sure our local listing is u2d after we changed it
        client.fetch_listings().await;
        let own_listings = client.get_own_listings();

        // Store data for next tick
        old_player = client.player.clone();
        old_own_listings = own_listings.clone();

        // Wait for next tick
        let sleep_for = SIMULATION_TICK_TIMER_IN_SECONDS - tick_timer.elapsed().as_secs();
        println!("Sleeping for {} seconds.", sleep_for);
        std::thread::sleep(std::time::Duration::from_secs(sleep_for));
    }
}
