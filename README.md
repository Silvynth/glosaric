# glosaric

Herramienta chiquita en Rust para trackear funciones públicas en un archivo Markdown (`docs/GLOSARIC.md`).
Útil para tener un mapa rápido de tu código y no perder tiempo haciendo `grep` a ciegas (para ti o para una IA).

> *Proyecto generado con asistencia de IA. No busca reemplazar herramientas oficiales de documentación ni nada por el estilo; solo extrae funciones y structs en una lista simple.*

---

## 📦 Instalación

Requiere tener Rust instalado:

```bash
git clone https://github.com/tu-usuario/glosaric.git
cd glosaric
cargo install --path .
```
*(El ejecutable queda instalado en `~/.cargo/bin/glosaric`)*.

---

## ⚡ Cómo se usa

Te paras en la carpeta de tu proyecto:

### 1. Crear el glosario
```bash
glosaric init .
```
Lee tu código y te genera `docs/GLOSARIC.md` con las funciones encontradas.

### 2. Buscar una función al vuelo
```bash
glosaric query "login" .
```

### 3. Actualizar cuando agregas código nuevo
```bash
glosaric sync . --backup
```
Agrega las funciones nuevas sin tocar ni borrar lo que ya habías explicado a mano.

---

## 🌐 Lenguajes que lee

Puedes verlos con:
```bash
glosaric languages
```
* **Rust** (`.rs`)
* **Dart / Flutter** (`.dart`)
* **Python** (`.py`)

*(Si tienes archivos en TypeScript, Go, etc., el programa te avisa que aún no tiene parser para ellos).*

---

## 📄 Licencia
MIT
