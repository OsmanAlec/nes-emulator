A nintendo entertainment system emulator that is capable of running NES 1.0 games.

# Instructions to use
1. Clone or download the repository
2. Make sure you have rust installed on your machine
3. In your root folder run the command 'cargo run'
4. And voila :)

There is a game already hard coded into the system: blaster.nes.
If you own any of the cartridges for older NES games and wish to try them out on this emulator, just change the file name in main.rs line 45 to the .nes file you place in the root folder.
I would recommend pacman.

This emulator currently supports NES 1.0 (mapper 0 / NROM) games.
Some homebrew or advanced mapper games may not run correctly.
Can find homebrew games here:
https://www.nesworld.com/article.php?system=nes&data=neshomebrew
