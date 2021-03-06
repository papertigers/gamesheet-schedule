use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub(crate) struct TeamAttributes {
    pub title: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct GameAttributes {
    pub scheduled_start_time: String,
    pub location: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct GameRelationships {
    pub home_team: TeamData,
    pub visitor_team: TeamData,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Data {
    pub id: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct TeamData {
    pub data: Data,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Included {
    ScheduledGames {
        attributes: GameAttributes,
        relationships: GameRelationships,
    },
    Teams {
        id: String,
        attributes: TeamAttributes,
    },
    #[serde(other)]
    _Ignored,
}

#[derive(Deserialize, Debug)]
pub(crate) struct GameSheet {
    pub included: Vec<Included>,
}
