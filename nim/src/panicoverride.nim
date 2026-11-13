proc panic(msg: string) {.noreturn.} =
  {.emit: "__builtin_unreachable();".}

proc rawoutput(msg: string) =
  discard

