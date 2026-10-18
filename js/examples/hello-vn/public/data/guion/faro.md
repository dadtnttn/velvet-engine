---
titulo: El faro de Brisa Baja
---

> Una historia corta para probar Velvet JS: tres escenas, dos elecciones y dos finales.

## puerto
@fondo bg.puerto
@musica music.pad
Las farolas de Brisa Baja se han apagado una a una. Desde el muelle solo se ve un resplandor torcido allá arriba, donde debería estar el faro.
**Tomé (preocupado):** ¡Por fin! ¿Eres la cartógrafa que mandó el gremio? Llegas justo a tiempo para ver cómo se apaga la costa.
**Tomé (preocupado):** La lente del faro se partió en la última tormenta. Sin luz, el barco de la noche no encuentra la bahía.
**Lupa (picara):** Si buscas vidrio de mar, tengo un trozo en la taberna. Te lo cambio por una buena historia, eso sí.
? ¿Qué haces?
- Subir al faro con Tomé, ahora mismo -> faro
- Pasar antes por la taberna a por el vidrio -> taberna

## taberna
@fondo bg.taberna
La taberna El Ancla Torcida huele a pan caliente y a sal. Lupa rebusca tras la barra y saca un paño con algo que brilla como agua quieta.
**Lupa (picara):** Vidrio de mar, de la marea más brava del año. Pesa menos de lo que parece, ¿verdad?
**Lupa (picara):** Me pagas con la historia de cómo acabaste en Brisa Baja. Siempre he querido saberla.
Guardas el vidrio en la bolsa, envuelto en el paño. Ya solo queda subir.
@var lente
@salta faro

## faro
@fondo bg.faro
La escalera de caracol suena bajo las botas como una campana vieja. Arriba, la lámpara está apagada y la niebla entra por las ventanas.
**Tomé (sonriente):** ¡Mira eso! Desde aquí se ven las luces de los barcos, esperando fuera de la bahía.
**Tomé (preocupado):** Si no les damos una señal, irán directos a los arrecifes. Tú decides cómo se la damos.
? ¿Cómo avisamos a los barcos?
- Encender la lámpara con el vidrio de mar [lente] -> fin {final = 1}
- Tocar la campana del faro, que se oye a una legua -> fin {final = 2}
