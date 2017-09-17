use chrono::{DateTime, NaiveDateTime, Utc};
use petronel::error::*;
use petronel::model::{BossImageUrl, RaidBoss, RaidBossMetadata};
use prost::Message;
use protobuf;
use redis::{self, Commands};
use std::collections::HashSet;

pub(crate) struct Cache {
    redis_connection: redis::Connection,
    legacy_bosses_key: String,
}

impl Cache {
    pub(crate) fn new<U>(url: U, legacy_bosses_key: String) -> Result<Self>
    where
        U: AsRef<str>,
    {
        let redis_connection = redis::Client::open(url.as_ref())
            .chain_err(|| "failed to start Redis client")?
            .get_connection()
            .chain_err(|| "failed to get Redis connection")?;

        Ok(Cache {
            redis_connection,
            legacy_bosses_key,
        })
    }

    pub(crate) fn get_legacy_bosses(&self) -> Result<Vec<RaidBossMetadata>> {
        let bytes: Vec<u8> = self.redis_connection
            .get(&self.legacy_bosses_key)
            .chain_err(|| "failed to get bosses from cache")?;

        let mut bosses_proto = protobuf::RaidBossesCacheItem::decode(bytes)
            .chain_err(|| "failed to parse bosses from cache")?
            .raid_bosses;

        let output = bosses_proto
            .drain(..)
            .map(|boss_proto| {
                let mut translations = HashSet::with_capacity(1);
                if let Some(translation) = boss_proto.translated_name {
                    translations.insert(translation.into());
                }

                let boss = RaidBoss {
                    name: boss_proto.name.into(),
                    level: boss_proto.level as i16,
                    image: boss_proto.image.map(BossImageUrl::from),
                    language: protobuf::convert::language_from_proto(boss_proto.language),
                    translations,
                };

                let last_seen = DateTime::<Utc>::from_utc(
                    NaiveDateTime::from_timestamp(boss_proto.last_seen, 0),
                    Utc,
                );

                RaidBossMetadata {
                    boss,
                    last_seen,
                    image_hash: None, // TODO
                }
            })
            .collect();

        Ok(output)
    }
}
