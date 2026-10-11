cmake_minimum_required(VERSION 3.15)
project(DengjenTashkeelCapiTestPackage LANGUAGES C)

find_package(dengjen-tashkeel-capi REQUIRED CONFIG)

add_executable(example example.c)
target_link_libraries(example PRIVATE dengjen-tashkeel-capi::dengjen-tashkeel-capi)
