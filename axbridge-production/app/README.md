# app/

**Tutaj umieść swój plik `app.ax`.**

```bash
# z katalogu głównego projektu:
cp /ścieżka/do/twojego/app.ax ./app/app.ax
```

Kompilator oczekuje dokładnie:

```
axbridge-production/app/app.ax
```

Po wstawieniu pliku:

```bash
axc check app/app.ax
axc build app/app.ax --target workers -o worker
axc run app/app.ax --target web
```
