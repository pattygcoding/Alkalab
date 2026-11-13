# Moore and von Neumann neighbour offsets.

const
  NEIGHBOURS8*: array[8, (int32, int32)] = [
    (-1'i32, -1'i32), (0'i32, -1'i32), (1'i32, -1'i32),
    (-1'i32, 0'i32), (1'i32, 0'i32),
    (-1'i32, 1'i32), (0'i32, 1'i32), (1'i32, 1'i32)
  ]
  NEIGHBOURS4*: array[4, (int32, int32)] = [
    (0'i32, -1'i32), (-1'i32, 0'i32), (1'i32, 0'i32), (0'i32, 1'i32)
  ]
