# lbp archive download tool
a somewhat crappy tool to download levels from [tamiya99's lbp level archive](https://archive.org/details/@tamiya99)

# usage
- download the [latest release](https://github.com/uhwot/lbp_archive_dl/releases/latest) and extract it
- download dry.db from [here](https://archive.org/download/dry23db) (2.5 GB in size, it will take a while), move it to the same folder as the executable
- open a terminal, cd to where the executable is and run:\
`./archive_dl bkp <level id>` (the level id is the `id` column in the `slot` table, *not* the rootLevel hash)
- move the level backup from the newly created `backups` folder and import it in the game!
- after that, look in `config.yml` and change whatever you feel like

# special thanks :)
- [aidan](https://github.com/ennuo) for writing [cwlib](https://github.com/ennuo/toolkit/tree/main/lib/cwlib) and reverse-engineering LBP to make this all possible
- [jvyden](https://github.com/jvyden) for hosting the archive assets on his [LittleBigRefresh](https://lbp.littlebigrefresh.com/) server, you should check it out :D
- [ugng](https://gitlab.com/osyu) for giving me PARAM.PFD serialization code in python, from which [this code](https://github.com/uhwot/lbp_archive_dl/blob/master/src/serializers/ps3/pfd.rs) is based off of