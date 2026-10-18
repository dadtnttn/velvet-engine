---
id: cabecera
sala: posada
titulo: La cabecera
categoria: escena
participantes: Lira, Nyxara, Nemi
ilustracion: lira_bano
hora: 10:30
vacio:
---
## inicio {quien=lira}
**Lira:** —Llegas a tiempo.
*La puerta se cierra.*
? ¿Qué le dices?
- Que sí -> fin {lira_confianza +1}
- Nada [lira_confianza >= 2] -> fin
