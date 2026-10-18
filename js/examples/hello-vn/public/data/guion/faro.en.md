---
title: The Lighthouse of Brisa Baja
---

> A short story to try Velvet JS: three scenes, two choices and two endings. Same structure as faro.md.

## puerto
@fondo bg.puerto
@musica music.pad
The streetlamps of Brisa Baja have gone out one by one. From the pier, all you can see is a crooked glow up there, where the lighthouse should be.
**Tomé (preocupado):** At last! Are you the cartographer the guild sent? You're just in time to watch the coast go dark.
**Tomé (preocupado):** The lighthouse lens cracked in the last storm. Without light, the night boat can't find the bay.
**Lupa (picara):** If you're after sea glass, I have a piece at the tavern. I'll trade it for a good story, mind you.
? What will you do?
- Climb the lighthouse with Tomé, right now -> faro
- Stop by the tavern for the glass first -> taberna

## taberna
@fondo bg.taberna
The Crooked Anchor smells of warm bread and salt. Lupa rummages behind the bar and pulls out a cloth with something that shines like still water.
**Lupa (picara):** Sea glass, from the wildest tide of the year. It weighs less than it looks, doesn't it?
**Lupa (picara):** Pay me with the story of how you ended up in Brisa Baja. I've always wanted to know.
You tuck the glass into your bag, wrapped in the cloth. Only the climb is left now.
@var lente
@salta faro

## faro
@fondo bg.faro
The spiral stairs ring under your boots like an old bell. Up above, the lamp is dark, and fog creeps in through the windows.
**Tomé (sonriente):** Look at that! From here you can see the lights of the boats, waiting outside the bay.
**Tomé (preocupado):** If we don't give them a signal, they'll sail straight onto the reefs. Your call how we do it.
? How do we warn the boats?
- Light the lamp with the sea glass [lente] -> fin {final = 1}
- Ring the lighthouse bell, which carries for a league -> fin {final = 2}
