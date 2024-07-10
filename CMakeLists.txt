cmake_minimum_required(VERSION 3.7)
project(OSC)

set(IMGUI_SOURCES lib/imgui/imgui.cpp lib/imgui/imgui_draw.cpp lib/imgui/imgui_widgets.cpp lib/imgui/imgui_demo.cpp lib/imgui/imgui_tables.cpp lib/imgui-sfml/imgui-SFML.cpp)
add_executable(OSC Load.cpp Main.cpp OSC.cpp Renderer.cpp Save.cpp SHA256.cpp ${IMGUI_SOURCES})

target_include_directories(OSC PUBLIC lib/imgui)
target_include_directories(OSC PUBLIC lib/imgui-sfml)
target_include_directories(OSC PUBLIC lib/SFML/include)

target_compile_options(OSC PUBLIC -include ${PROJECT_SOURCE_DIR}/Common.h)

add_subdirectory(lib/SFML)

target_link_libraries(OSC sfml-graphics sfml-window sfml-system sfml-network sfml-audio)
target_link_libraries(OSC GL)