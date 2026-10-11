# Sonidos de Vida Sin Filtro

Todos los efectos y la música se sintetizan desde cero con numpy (no hay
muestras de terceros). Para regenerarlos:

```
python3 make_sfx.py   ../../assets/sounds
python3 make_music.py ../../assets/sounds            # todas las pistas
python3 make_music.py ../../assets/sounds bgm_life   # solo una
```

- `synth_lib.py`: instrumentos (piano eléctrico FM, piano, vibráfono, campanitas,
  marimba, bajo, pizzicato, pad, flauta, cuerda pulsada), batería, filtros,
  reverb y el secuenciador en bucle (la cola de cada nota y de la reverb vuelve
  al inicio, así que las pistas se repiten sin costura).
- `make_sfx.py`: los `sfx_*` (menús, resultados, dinero, amor, cumpleaños,
  muerte, sello, celular, rango, ruptura, mes).
- `make_music.py`: `bgm_title` (portada), `bgm_life` (vida adulta),
  `bgm_child` (infancia), `bgm_night` (prisión) y `bgm_death` (despedida).

Salida: WAV mono de 16 bits a 22 050 Hz, declarados en `app_manifest()`
(`main.vel`). El juego los pide desde `sonido.vel`.
