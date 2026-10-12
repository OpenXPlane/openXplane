# Меню openXplane

В нативных `view`, `airport-view` и `fly` добавлен интерфейс поверх wgpu-сцены.

- `Alt` (Option на macOS) показывает/скрывает верхнюю панель. Наведение на верхний край тоже показывает её.
- `Esc` открывает Main Menu, повторный Esc возвращает к сцене; из подстраницы возвращает в Main Menu.
- Обратная кавычка открывает Flight Configuration.
- Мышь выбирает пункты; Tab/стрелки вверх/вниз перемещают выделение, Enter/Space активируют.
- Выход: Quit или закрытие окна. Esc больше не завершает приложение.

File содержит Main Menu, Flight Configuration, Settings, Quit. Flight — Resume Flight,
Pause / Resume, Restart Flight. View — Default View, Free Camera, Flight Telemetry.
Developer содержит About; Plugins пока показывает неактивный пункт No plugins loaded.

Main Menu содержит Resume Flight, New Flight, Settings, Keyboard, About, Quit.
В `fly` **New Flight позволяет выбрать другой ACF и аэропорт**. Клик по имени самолёта
циклически выбирает следующий найденный ACF (порядок путей стабильный). Клик по Airport активирует
ввод идентификатора: Backspace удаляет символы, буквы/цифры вводят новый код, Enter завершает ввод.
Start New Flight загружает аэропорт и самолёт в существующее окно и начинает полёт с первой полосы.
При ошибке загрузки сообщение появляется в меню, предыдущая сцена и модель сохраняются.
Чтение файлов и загрузка текстур пока синхронные: окно может ненадолго перестать отвечать.
Список аэропортов с поиском, выбор полосы и thumbnails пока отсутствуют.
В статических просмотрах конфигурация не имеет каталога установки; отдельного запуска полёта из них нет. Settings переключает
клавиатурный штурвал, паузу и показ телеметрии; Keyboard показывает основные сочетания клавиш.
В статическом просмотре пункты, требующие полёта, отключены.

Модальная страница временно останавливает физику. При закрытии Esc ручная пауза P сохраняется;
Resume Flight снимает её. Верхняя панель сама по себе не останавливает полёт.
После Alt клавиатура направлена в панель; клик за пределами меню возвращает управление самолёту.
Открытые страницы и выпадающие меню перехватывают клавиатуру, их области перехватывают мышь.
Открытие меню отпускает удерживаемый штурвал и обычный тормоз; положение газа, триммеров,
закрылков и фиксация максимального тормоза сохраняются.

Это самостоятельная реализация меню, а не точная копия UI X-Plane. Основания и ограничения:
[research/UI.md](../research/UI.md).

## Запуск и проверка

```sh
cargo run --offline -- fly Xplane12 KSEA "Xplane12/Cessna 172 SP/Cessna_172SP.acf"
cargo run --offline -- ui-preview esc /tmp/openxplane-esc.png
cargo run --offline -- ui-preview alt /tmp/openxplane-alt.png
cargo run --offline -- ui-preview settings /tmp/openxplane-settings.png
OPENXPLANE_UI_SMOKE=esc cargo run --offline -- fly Xplane12 KSEA "Xplane12/Cessna 172 SP/Cessna_172SP.acf" --smoke
```

`ui-preview config /tmp/config.png` показывает конфигурацию на демонстрационных данных.
`ui-preview` сохраняет растровый предпросмотр на одноцветном фоне (не скриншот GPU).
`OPENXPLANE_UI_SMOKE=alt|esc|settings` задаёт начальную страницу для диагностики; `--smoke`
закрывает окно после нескольких кадров. Семь тестов `ui::tests` проверяют переходы,
перехват, неактивные пункты, клавиатуру, размер, Retina-координаты и ввод конфигурации. GPU smoke проверяет
создание pipeline и вывод, но не реальные пользовательские события клавиатуры/мыши.

Используется системный TTF-шрифт Arial/Segoe UI/DejaVu/Liberation; файлы шрифтов не распространяются
с движком. Если подходящий шрифт не найден, задайте `OPENXPLANE_UI_FONT=/path/font.ttf`.

Проверка загрузки новой сессии на Metal:

```sh
OPENXPLANE_UI_RELOAD_SMOKE=KOAK cargo run --offline -- fly Xplane12 KSEA "Xplane12/Cessna 172 SP/Cessna_172SP.acf" --smoke
OPENXPLANE_UI_RELOAD_SMOKE=ZZZZZZZZ cargo run --offline -- fly Xplane12 KSEA "Xplane12/Cessna 172 SP/Cessna_172SP.acf" --smoke
```

В режиме `--smoke` эта переменная выбирает следующий ACF и указанный аэропорт, затем проходит
через тот же путь загрузки, что Start New Flight. Во втором примере ожидается отказ с сохранением
предыдущей сессии. Режим не генерирует реальные события мыши/клавиатуры.
