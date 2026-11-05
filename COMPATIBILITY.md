# Compatibility with Civil 3D

What civil3drust does with each Civil 3D object, listed the way Civil 3D
2024 lists them: **Prospector** (objects in the drawing) and **Settings**
(styles and settings per feature). Code references are to `src/`.

| status | meaning |
|---|---|
| **Supported** | read, edited and created |
| **Partial** | some of it is read or written; the row says what |
| **Read-only** | decoded, never written |
| **Referenced** | linked by handle from something civil3drust writes; its content is not decoded |
| **Passthrough** | not decoded; acadrust keeps its records as opaque bytes and writes them back unchanged when the drawing is opened with `raw::open` (D-033). Checked only on the sampled drawings, which contain few objects of other kinds |
| **Rejected** | tried; Civil 3D refuses what civil3drust writes |

Anything not listed is Passthrough.

## Prospector (objects)

| Civil 3D item | code | status |
|---|---|---|
| Points | — | Passthrough |
| Point Groups | — | Passthrough |
| Surfaces | — | Passthrough |
| **Alignments** | `aecc_alignment`, `aecc_elements` (read); `aecc_create::create_alignment`; `aecc_delete::delete_alignment` | Partial — see the alignment rows below |
| &nbsp;&nbsp;Centerline Alignments | `aecc_build::TYPE_CENTERLINE` (1), `alignment_type` / `set_alignment_type` | Supported |
| &nbsp;&nbsp;Offset Alignments | — (API value 2) | Rejected as a bare type change: S61 opened with errors, RECOVER removed it (S63). Offset parameters and parent link not sampled |
| &nbsp;&nbsp;Curb Return Alignments | — (API value 3) | Rejected the same way (S61, S63) |
| &nbsp;&nbsp;Rail Alignments | — (API value 5) | Rejected the same way (S61, S63) |
| &nbsp;&nbsp;Rail Diverted Alignments | — (no API value known) | Passthrough |
| &nbsp;&nbsp;Miscellaneous Alignments | `aecc_build::TYPE_MISCELLANEOUS` (4; API "Utility") | Supported |
| Feature Lines | — | Passthrough |
| Sites | `aecc_site::sites`, `membership` | Read-only — new alignments are siteless; deleting is siteless only (D-038, D-062) |
| Turnouts and Crossovers | — | Passthrough |
| &nbsp;&nbsp;Crossover Groups | — | Passthrough |
| &nbsp;&nbsp;Turnouts | — | Passthrough |
| Catchments | — | Passthrough |
| Pipe Networks | — | Passthrough |
| &nbsp;&nbsp;Networks | — | Passthrough |
| &nbsp;&nbsp;Interference Checks | — | Passthrough |
| Pressure Networks | — | Passthrough |
| Bridges | — | Passthrough |
| Corridors | — | Passthrough |
| Assemblies | — | Passthrough |
| &nbsp;&nbsp;Unassigned Subassemblies | — | Passthrough |
| Subassemblies | — | Passthrough |
| &nbsp;&nbsp;Other Subassemblies | — | Passthrough |
| Intersections | — | Passthrough |
| Survey | — | Passthrough |
| &nbsp;&nbsp;Networks | — | Passthrough |
| &nbsp;&nbsp;Figures | — | Passthrough |
| View Frame Groups | — | Passthrough |

### Inside an alignment

Children Prospector shows under each alignment, and the parts of the
alignment record itself.

| Civil 3D item | code | status |
|---|---|---|
| Name, description | `string_stream::set_name`, `set_description` | Supported |
| Lines (fixed, floating, free) | `aecc_elements::decode_elements`; `aecc_alignment_edit::set_tangent_length`, `set_pis` | Supported |
| Curves (fixed, floating, free, three-point) | `aecc_elements`; `aecc_arc_edit::set_free_arc_radius`; `aecc_element_edit::add_free_curve` | Supported for free curves; other curve kinds read-only |
| Removing / replacing elements | `aecc_element_edit::remove_element`, `replace_elements` | Partial — refused on alignments with spirals |
| Spirals (line+spiral, spiral, free spiral) | `aecc_elements` | Read-only — editing refused |
| Spiral groups (SCS, SC, SL …) | `aecc_elements`, `aecc_layout_solve`; written as typed parameters (D-059) | Partial — read, re-encoded when copied; not editable |
| Spiral types | `spiral_type` | Read-only — 7 shapes drawn; NSW Cubic Parabola (264) drawn as a clothoid (FRT-1) |
| Start station | `aecc_station::start_station`, `set_start_station`, `move_start_station` | Supported |
| Station equations | — | Passthrough (not decoded) |
| Design speeds | `aecc_design_speed::design_speeds`, `set_design_speeds`, `shift_design_speeds`, `clear_design_speeds` | Partial — refused on spiral groups |
| Design criteria check boxes, constraint editing | `aecc_criteria::criteria`, `set_criteria`, `constraint_editing`, `set_constraint_editing` | Partial — refused on spiral groups |
| PI display | `aecc_build::pi_display`, `set_pi_display` | Supported |
| Display (proxy graphics) | `proxy_graphics`; `aecc_build::replace_graphics`, `strip_graphics` | Supported — written for lines and arcs |
| Superelevation | — | Passthrough |
| Profiles | — | Passthrough |
| Profile Views | — | Passthrough |
| Sample Line Groups | — | Passthrough |
| Station equation label group | `aecc_create` (`AECC_ALIGNMENT_STAEQU_LABEL_GROUP`) | Partial — optionally created with a new alignment |
| Other label groups (station, minor station, geometry point, design speed) | — | Passthrough — not created for new alignments |
| Site companion and registry entry | `aecc_topology`, `aecc_registry` | Supported — rebuilt on create and delete (D-064) |

## Settings (styles and settings)

| Civil 3D item | code | status |
|---|---|---|
| General | — | Passthrough |
| Point | — | Passthrough |
| Surface | — | Passthrough |
| Parcel | — | Passthrough |
| Grading | — | Passthrough |
| **Alignment** | | Partial — see below |
| &nbsp;&nbsp;Alignment styles | `aecc_alignment_refs::alignment_styles`, `set_alignment_style`; `aecc_style::components`, `set_component_visible` | Partial — listed, assigned, component visibility set; other style content not decoded; never created (D-015) |
| &nbsp;&nbsp;Label set styles | — | Referenced — the alignment's handle to it is kept as read or as in the template; not decoded |
| &nbsp;&nbsp;Design check sets | `aecc_alignment_refs` (`check_set`) | Referenced |
| &nbsp;&nbsp;Label styles, design checks, command settings | — | Passthrough |
| Profile | — | Passthrough |
| Profile View | — | Passthrough |
| Superelevation View | — | Passthrough |
| Cant View | — | Passthrough |
| Sample Line | — | Passthrough |
| Section | — | Passthrough |
| Section View | — | Passthrough |
| Mass Haul Line | — | Passthrough |
| Mass Haul View | — | Passthrough |
| Rail | — | Passthrough |
| Rail Turnout | — | Passthrough |
| Catchment | — | Passthrough |
| Pipe Network | — | Passthrough |
| Pipe | — | Passthrough |
| Structure | — | Passthrough |
| Pressure Network | — | Passthrough |
| Pressure Pipe | — | Passthrough |
| Fitting | — | Passthrough |
| Appurtenance | — | Passthrough |
| Bridge | — | Passthrough |
| Corridor | — | Passthrough |
| Intersection | — | Passthrough |
| Assembly | — | Passthrough |
| Subassembly | — | Passthrough |
| Quantity Takeoff | — | Passthrough |
| Survey | — | Passthrough |
| View Frame Group | — | Passthrough |
| View Frame | — | Passthrough |
| Match Line | — | Passthrough |
| Building Site | — | Passthrough |
