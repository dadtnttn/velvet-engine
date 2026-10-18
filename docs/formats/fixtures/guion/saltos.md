---
id: saltos
---
## fiesta {quien=greta, una_vez, si=noche}
**Greta:** —¡Música!
@var fiesta_vista

## saludo {quien=greta}
@si greta_confianza >= 3 -> confianza
**Greta:** —Otra vez por aquí, viajera.
@var saludos +1

## confianza
**Greta:** —Ya eres de la casa.

## cuenta_atras {en=reloj}
@var reloj_visto +1
@si reloj_visto >= 2 -> campanada
**Greta:** —Aún no es medianoche.
@fin

## campanada
**Greta:** —¡Medianoche!
@fin
