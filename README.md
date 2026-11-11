# 📖 Glosaric CLI (`glosaric`)

> **Herramienta simple para trackear funciones públicas y tener un mapa rápido de tu código.**  
> *Nota de honestidad: Este proyecto fue generado y estructurado con asistencia de IA.*

---

## 💡 ¿Qué es y qué NO es?

* **NO busca reemplazar nada:** No viene a sustituir `cargo doc`, ni JSDoc, ni Doxygen, ni Swagger, ni la documentación oficial profunda de tu proyecto.
* **Solo hace una cosa sencilla:** Escanea tu código fuente, pesca las funciones públicas (`pub fn`, `def`, métodos de clases), structs y enums, y te los anota en un solo archivo Markdown (`docs/GLOSARIC.md`).
* **¿Para qué sirve en la práctica?** Para tener un "mapa de ruta" rápido en una sola página. Cuando tú (o una IA como Cursor, Copilot o Claude) necesitan saber qué funciones existen en el proyecto, abren ese archivo con `Ctrl + F` y lo encuentran en 5 segundos en vez de perder tiempo tirando `grep`s a ciegas por 40 carpetas.

---

## 🌐 Lenguajes que lee actualmente
Puedes consultar en cualquier momento con `glosaric languages`:
* 🦀 **Rust** (`.rs`): funciones públicas, structs, enums, tipos.
* 🎯 **Dart / Flutter** (`.dart`): clases, widgets, métodos públicos y comentarios `//`.
* 🐍 **Python** (`.py`): clases, funciones y docstrings `"""..."""`.

*(Si corres la herramienta en un proyecto con archivos en TypeScript, Go, C++, etc., la herramienta te avisará con honestidad cuántos archivos de esos lenguajes encontró para que sepas que aún no los puede leer).*

---

## 🚀 Uso Rápido

### 1. Generar el mapa inicial
Te paras en la carpeta de tu proyecto y corres:
```bash
glosaric init .
```
Te creará `docs/GLOSARIC.md` agrupado por carpetas con este formato de tres columnas:
```markdown
- `funcion(params)` | Firma técnica / tipos | Explicación o comentario
```

*(Si usas Obsidian y quieres que te lo guarde directo en tu bóveda, le puedes pasar `--vault`).*

### 2. Buscar algo al vuelo sin abrir editores
```bash
glosaric query "login" .
```
Busca en milisegundos en el glosario por nombre de función, por tipo o por palabra clave.

### 3. Sincronizar cuando agregas código nuevo
Si creaste funciones nuevas y quieres agregarlas al glosario **sin que te borre las explicaciones que ya habías escrito**:
```bash
glosaric sync . --backup
```
Agrega lo nuevo al final, te hace una copia de respaldo `.backup.md` por seguridad y no toca lo que ya tenías documentado.

### 4. Modo chequeo (opcional para pre-commit o CI)
```bash
glosaric check .
```
Te avisa si tienes funciones públicas en el código que no has anotado en el glosario.

---

## 📄 Licencia
MIT / Apache 2.0
