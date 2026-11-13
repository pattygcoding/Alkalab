# One grid cell: element, variant, lifetime, momentum and residue.
import element
# -----------------------------------------------------------------------------
# cell - the single unit of state
# -----------------------------------------------------------------------------

type
  Cell* = object
    ## What the cell is made of, its element-specific state, remaining life,
    ## vertical momentum and the element it decays into.
    element*: Element
    variant*: uint8
    life*: uint16
    vel*: int8
    residue*: Element

func cellEmpty*(): Cell {.inline.} =
  Cell(element: Element.Empty, variant: VARIANT_DEFAULT, life: 0, vel: 0,
       residue: Element.Empty)

func cellRaw*(element: Element, variant: uint8, life: uint16, vel: int8,
              residue: Element): Cell {.inline.} =
  Cell(element: element, variant: variant, life: life, vel: vel,
       residue: residue)

func newCell*(element: Element): Cell {.inline.} =
  let p = element.props
  Cell(element: element, variant: VARIANT_DEFAULT, life: p.life, vel: 0,
       residue: p.residue)

func cellTransient*(element: Element, life: uint16, residue: Element): Cell {.inline.} =
  Cell(element: element, variant: VARIANT_DEFAULT, life: life, vel: 0,
       residue: residue)

func isElement*(c: Cell, element: Element): bool {.inline.} = c.element == element
func isEmpty*(c: Cell): bool {.inline.} = c.element == Element.Empty
func isWet*(c: Cell): bool {.inline.} = c.variant == VARIANT_WET

func tickLifetime*(c: var Cell): bool {.inline.} =
  ## Moves the cell into its residue and returns true when it expired.
  if c.life == 0:
    return false
  dec c.life
  if c.life == 0:
    c = newCell(c.residue)
    return true
  false

