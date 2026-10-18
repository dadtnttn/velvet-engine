---
id: posadera
sala: taberna
titulo: La posada del Cuervo
---
# La posada del Cuervo
> Notes for the writer: this line is ignored.

## llegada {en=puerta}
@titulo LA POSADA DEL CUERVO | DE NOCHE
**Greta:** —Pasa, viajera, que llueve a cántaros.
*La puerta cruje al cerrarse.*
(Huele a pan recién hecho. Podría quedarme.)
La posadera limpia una jarra con el delantal.
? ¿Qué le ofreces?
- Un caldo caliente -> caldo {greta_confianza +1}
- Una habitación [oro >= 5] -> habitacion {oro -5}
- Nada, sigo camino -> fin

## caldo
**Greta (sonriendo):** —Enseguida. Siéntate junto al fuego.
@var caldo_servido
**Bruno:** —Yo también quiero uno, Greta.

## habitacion
**Greta:** —Arriba, la última puerta. Son cinco monedas.
@fin
