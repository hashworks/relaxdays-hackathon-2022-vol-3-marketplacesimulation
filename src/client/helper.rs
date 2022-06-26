use super::{types::Listing, Client};

impl Client {
    pub fn get_tags_for_article_id(&self, article_id: usize) -> Vec<(usize, Vec<usize>)> {
        let article_tags = self
            .articles
            .iter()
            .filter(|article| article.id == article_id)
            .map(|article| article.tags.clone())
            .next();
        if article_tags.is_none() {
            return Vec::new();
        }

        let article_tags = article_tags.unwrap();

        let mut result = Vec::new();

        for article_tag in article_tags {
            let tag_and_similar_tags = self
                .tags
                .iter()
                .filter(|tag| tag.id == article_tag)
                .map(|tag| (tag.id, tag.similar_tags.clone()))
                .next();
            result.push(match tag_and_similar_tags {
                Some(tag_and_similar_tags) => tag_and_similar_tags,
                None => (article_tag, Vec::new()),
            });
        }

        result
    }

    // Filter out other players, we don't trust their data anyway
    // Filter out our own bedazzlement listings
    pub fn get_own_listings(&self) -> Vec<Listing> {
        self.listings
            .iter()
            .filter(|listing| {
                listing.player == self.player.id
                    && !self.bedazzlement_listings.contains(&listing.id)
            })
            .cloned()
            .collect::<Vec<_>>()
    }

    // Filter out ourselves
    // Filter out empty listings
    // Filter out our own bedazzlement listings
    pub fn get_other_listings(&self) -> Vec<Listing> {
        self.listings
            .iter()
            .filter(|listing| {
                listing.player != self.player.id
                    && listing.count > 0
                    && !self.bedazzlement_listings.contains(&listing.id)
            })
            .cloned()
            .collect::<Vec<_>>()
    }
}
