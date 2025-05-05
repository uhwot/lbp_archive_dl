const fn lams(tag: &str) -> u32 {
    let tag = tag.as_bytes();

    let mut v0: u64 = 0;
    let mut v1: u64 = 0xC8509800;

    let mut i = 31;
    loop {
        let char = match i < tag.len() {
            true => tag[i] as u64,
            false => 0x20,
        };
        v0 = v0.wrapping_mul(0x1b).wrapping_add(char);
        if i == 0 { break; }
        i -= 1;
    }

    if tag.len() > 32 {
        v1 = 0;
        i = 63;
        loop {
            let char = match i < tag.len() {
                true => tag[i] as u64,
                false => 0x20,
            };
            v1 = v1.wrapping_mul(0x1b).wrapping_add(char);
            if i == 32 { break; }
            i -= 1;
        }
    }

    let result = v0.wrapping_add(v1.wrapping_mul(0xDEADBEEF));
    (result & 0xFFFFFFFF) as u32
}

pub const LABEL_LAMS_KEY_IDS: [u32; 85] = [
    lams("LABEL_SinglePlayer"),
    lams("LABEL_RPG"),
    lams("LABEL_Multiplayer"),
    lams("LABEL_SINGLE_PLAYER"),
    lams("LABEL_Musical"),
    lams("LABEL_Artistic"),
    lams("LABEL_Funny"),
    lams("LABEL_Scary"),
    lams("LABEL_Easy"),
    lams("LABEL_Challenging"),
    lams("LABEL_Long"),
    lams("LABEL_Quick"),
    lams("LABEL_Time_Trial"),
    lams("LABEL_Seasonal"),
    lams("LABEL_16_Bit"),
    lams("LABEL_8_Bit"),
    lams("LABEL_Homage"),
    lams("LABEL_Technology"),
    lams("LABEL_Pinball"),
    lams("LABEL_Movie"),
    lams("LABEL_Sticker_Gallery"),
    lams("LABEL_Costume_Gallery"),
    lams("LABEL_Music_Gallery"),
    lams("LABEL_Prop_Hunt"),
    lams("LABEL_Hide_And_Seek"),
    lams("LABEL_Hangout"),
    lams("LABEL_Driving"),
    lams("LABEL_Defence"),
    lams("LABEL_Party_Game"),
    lams("LABEL_Mini_Game"),
    lams("LABEL_Card_Game"),
    lams("LABEL_Board_Game"),
    lams("LABEL_Arcade_Game"),
    lams("LABEL_Social"),
    lams("LABEL_Sci_Fi"),
    lams("LABEL_3rd_Person"),
    lams("LABEL_1st_Person"),
    lams("LABEL_CO_OP"),
    lams("LABEL_TOP_DOWN"),
    lams("LABEL_Retro"),
    lams("LABEL_Tutorial"),
    lams("LABEL_SurvivalChallenge"),
    lams("LABEL_Strategy"),
    lams("LABEL_Story"),
    lams("LABEL_Sports"),
    lams("LABEL_Shooter"),
    lams("LABEL_Race"),
    lams("LABEL_Platform"),
    lams("LABEL_Puzzle"),
    lams("LABEL_Gallery"),
    lams("LABEL_Fighter"),
    lams("LABEL_Competitive"),
    lams("LABEL_Cinematic"),
    lams("LABEL_FLOATY_FLUID_NAME"),
    lams("LABEL_HOVERBOARD_NAME"),
    lams("LABEL_SPRINGINATOR"),
    lams("LABEL_SACKPOCKET"),
    lams("LABEL_QUESTS"),
    lams("LABEL_INTERACTIVE_STREAM"),
    lams("LABEL_WALLJUMP"),
    lams("LABEL_MEMORISER"),
    lams("LABEL_HEROCAPE"),
    lams("LABEL_ATTRACT_TWEAK"),
    lams("LABEL_ATTRACT_GEL"),
    lams("LABEL_Paint"),
    lams("LABEL_Movinator"),
    lams("LABEL_Brain_Crane"),
    lams("LABEL_Water"),
    lams("LABEL_Vehicles"),
    lams("LABEL_Sackbots"),
    lams("LABEL_PowerGlove"),
    lams("LABEL_Paintinator"),
    lams("LABEL_LowGravity"),
    lams("LABEL_MagicBag"),
    lams("LABEL_JumpPads"),
    lams("LABEL_GrapplingHook"),
    lams("LABEL_Glitch"),
    lams("LABEL_Explosives"),
    lams("LABEL_DirectControl"),
    lams("LABEL_Collectables"),
    lams("LABEL_CREATED_CHARACTERS"),
    lams("LABEL_SACKBOY"),
    lams("LABEL_SWOOP"),
    lams("LABEL_TOGGLE"),
    lams("LABEL_ODDSOCK"),
];

pub const LBP2_LABELS: [u32; 46] = [
    lams("LABEL_SinglePlayer"),
    lams("LABEL_Multiplayer"),
    lams("LABEL_Quick"),
    lams("LABEL_Long"),
    lams("LABEL_Challenging"),
    lams("LABEL_Easy"),
    lams("LABEL_Scary"),
    lams("LABEL_Funny"),
    lams("LABEL_Artistic"),
    lams("LABEL_Musical"),
    lams("LABEL_Intricate"),
    lams("LABEL_Cinematic"),
    lams("LABEL_Competitive"),
    lams("LABEL_Fighter"),
    lams("LABEL_Gallery"),
    lams("LABEL_Puzzle"),
    lams("LABEL_Platform"),
    lams("LABEL_Race"),
    lams("LABEL_Shooter"),
    lams("LABEL_Sports"),
    lams("LABEL_Story"),
    lams("LABEL_Strategy"),
    lams("LABEL_SurvivalChallenge"),
    lams("LABEL_Tutorial"),
    lams("LABEL_Retro"),
    lams("LABEL_Collectables"),
    lams("LABEL_DirectControl"),
    lams("LABEL_Explosives"),
    lams("LABEL_Glitch"),
    lams("LABEL_GrapplingHook"),
    lams("LABEL_JumpPads"),
    lams("LABEL_MagicBag"),
    lams("LABEL_LowGravity"),
    lams("LABEL_Paintinator"),
    lams("LABEL_PowerGlove"),
    lams("LABEL_Sackbots"),
    lams("LABEL_Vehicles"),
    lams("LABEL_Water"),
    lams("LABEL_Brain_Crane"),
    lams("LABEL_Movinator"),
    lams("LABEL_Paint"),
    lams("LABEL_ATTRACT_GEL"),
    lams("LABEL_ATTRACT_TWEAK"),
    lams("LABEL_HEROCAPE"),
    lams("LABEL_MEMORISER"),
    lams("LABEL_WALLJUMP"),
];