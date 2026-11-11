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
| &nbsp;&nbsp;Offset Alignments | — (API value 2) | Read (S255, S256): elements (offset tangents and arcs; offset spirals as part of the parent + offset), parent (also after it is deleted), offset and side, station ranges; widenings (regions; linear, curve – line – curve, curve – reverse curve and curve – curve – reverse curve transitions, also on arcs and spirals, S257, S258); parts of the parameters open (MIST19); not written. Rejected as a bare type change: S61 opened with errors, RECOVER removed it (S63) |
| &nbsp;&nbsp;Curb Return Alignments | — (API value 3) | Elements (circular fillet, three centered arc, spiral-curve-spiral, all three reverse-spiral types; S283–S289; after input edits, incl. a fillet from a curve, S290–S294), type, speeds, dynamic/static (S287), the two inputs read (S283, a production drawing); the connection parameters after the type partly read (MIST20); not written. Rejected as a bare type change (S61, S63) |
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
| Station equations | `aecc_station_refs::station_equations`, `display_station`, `set_station_equations` | Supported (S250–S254, D-070; also inside JSON groups, S253) — stations with equations applied; (S243, S247, S248: raw back, back, ahead, Increase/Decrease, comment and number strings; 18 leading bits a fixed marker, MIST18); kept when speeds are rewritten |
| Masking regions | `aecc_build::masking_regions`, `aecc_station_refs::set_masking_regions` | Supported (S250–S254, D-070; also inside JSON groups, S253; S243, S246: stations, Mask check box, locks derived as Civil 3D does (D-068), comment string); kept when speeds are rewritten |
| Design speeds | `aecc_design_speed::design_speeds`, `set_design_speeds`, `shift_design_speeds`, `clear_design_speeds` | Partial — speeds on fixed spirals written (S243); refused on spiral groups; start-station moves refused with station equations or masking |
| Design criteria check boxes, constraint editing | `aecc_criteria::criteria`, `set_criteria`, `constraint_editing`, `set_constraint_editing` | Partial — refused on spiral groups |
| PI display | `aecc_build::pi_display`, `set_pi_display` | Supported |
| Display (proxy graphics) | `proxy_graphics`; `aecc_build::replace_graphics`, `strip_graphics` | Supported — written for lines and arcs |
| Superelevation | — | Passthrough |
| Profiles | Layout profiles read: fixed, floating and free tangents; free vertical curves of every type (parabola by length/K, circular by length/radius/through point, asymmetric); fixed parabolas (three points, two points + K or radius, two points + grade); floating parabolas (point + K or radius, point + grade); Profile Locking settings; links to alignment and views (S298–S310) | Partial |
| Profile Views | Links to alignment and profiles (S304) | Partial |
| Sample Line Groups | — | Passthrough |
| Station equation label group | `aecc_create` (`AECC_ALIGNMENT_STAEQU_LABEL_GROUP`) | Partial — optionally created with a new alignment |
| Other label groups (station, minor station, geometry point, design speed) | — | Passthrough — not created for new alignments |
| Site companion and registry entry | `aecc_topology`, `aecc_registry` | Supported — rebuilt on create and delete (D-064) |

## Settings (styles and settings)

Listed as Civil 3D 2024 shows the Settings tree. Every item is a
collection of objects in the drawing (styles, sets, rule sets, command
defaults); a style's own sub-styles are not listed.

### General

| Civil 3D item | code | status |
|---|---|---|
| Multipurpose Styles | — | Passthrough |
| &nbsp;&nbsp;Marker Styles | — | Passthrough |
| &nbsp;&nbsp;Feature Line Styles | — | Passthrough |
| &nbsp;&nbsp;Slope Pattern Styles | — | Passthrough |
| &nbsp;&nbsp;Projection Styles | — | Passthrough |
| &nbsp;&nbsp;Code Set Styles | — | Passthrough |
| &nbsp;&nbsp;Link Styles | — | Passthrough |
| &nbsp;&nbsp;Shape Styles | — | Passthrough |
| Label Styles | — | Passthrough |
| &nbsp;&nbsp;Note | — | Passthrough |
| &nbsp;&nbsp;Line | — | Passthrough |
| &nbsp;&nbsp;Curve | — | Passthrough |
| &nbsp;&nbsp;Marker | — | Passthrough |
| &nbsp;&nbsp;Link | — | Passthrough |
| &nbsp;&nbsp;Shape | — | Passthrough |
| Commands | — | Passthrough |

### Point

| Civil 3D item | code | status |
|---|---|---|
| User-Defined Property Classifications | — | Passthrough |
| Point Styles | — | Passthrough |
| Label Styles | — | Passthrough |
| Point File Formats | — | Passthrough |
| Description Key Sets | — | Passthrough |
| External Data References | — | Passthrough |
| Table Styles | — | Passthrough |
| Commands | — | Passthrough |

### Surface

| Civil 3D item | code | status |
|---|---|---|
| Surface Styles | — | Passthrough |
| Label Styles | — | Passthrough |
| &nbsp;&nbsp;Contour | — | Passthrough |
| &nbsp;&nbsp;Slope | — | Passthrough |
| &nbsp;&nbsp;Spot Elevation | — | Passthrough |
| &nbsp;&nbsp;Watershed | — | Passthrough |
| Table Styles | — | Passthrough |
| &nbsp;&nbsp;Direction | — | Passthrough |
| &nbsp;&nbsp;Elevation | — | Passthrough |
| &nbsp;&nbsp;Slope | — | Passthrough |
| &nbsp;&nbsp;Slope Arrow | — | Passthrough |
| &nbsp;&nbsp;Contour | — | Passthrough |
| &nbsp;&nbsp;Watershed | — | Passthrough |
| &nbsp;&nbsp;User-Defined Contour | — | Passthrough |
| Commands | — | Passthrough |

### Parcel

| Civil 3D item | code | status |
|---|---|---|
| User-Defined Property Classifications | — | Passthrough |
| Parcel Styles | — | Passthrough |
| Label Styles | — | Passthrough |
| &nbsp;&nbsp;Area | — | Passthrough |
| &nbsp;&nbsp;Line | — | Passthrough |
| &nbsp;&nbsp;Curve | — | Passthrough |
| Table Styles | — | Passthrough |
| &nbsp;&nbsp;Line | — | Passthrough |
| &nbsp;&nbsp;Curve | — | Passthrough |
| &nbsp;&nbsp;Segment | — | Passthrough |
| &nbsp;&nbsp;Area | — | Passthrough |
| Commands | — | Passthrough |

### Grading

| Civil 3D item | code | status |
|---|---|---|
| Grading Styles | — | Passthrough |
| Grading Criteria Sets | — | Passthrough |
| Commands | — | Passthrough |

### Alignment

| Civil 3D item | code | status |
|---|---|---|
| **Alignment** | — | Partial — see the rows below |
| Alignment Styles | `aecc_alignment_refs::alignment_styles`, `set_alignment_style`; `aecc_style::components`, `set_component_visible` | Partial — listed, assigned to an alignment, component visibility set; other style content not decoded; never created (D-015) |
| Design Checks | — | Passthrough |
| &nbsp;&nbsp;Design Check Sets | `aecc_alignment_refs` (`check_set`); `aecc_create` | Referenced — read; kept on a created alignment when the target drawing has it, otherwise cleared with its check box; content not decoded |
| &nbsp;&nbsp;Line | — | Passthrough |
| &nbsp;&nbsp;Curve | — | Passthrough |
| &nbsp;&nbsp;Spiral | — | Passthrough |
| &nbsp;&nbsp;Superelevation | — | Passthrough |
| &nbsp;&nbsp;Tangent Intersection | — | Passthrough |
| Label Styles | — | Passthrough |
| &nbsp;&nbsp;Label Sets | — | Referenced — the alignment's link is kept as read or as in the template; not decoded |
| &nbsp;&nbsp;Station | — | Passthrough |
| &nbsp;&nbsp;Station Offset | — | Passthrough |
| &nbsp;&nbsp;Line | — | Passthrough |
| &nbsp;&nbsp;Curve | — | Passthrough |
| &nbsp;&nbsp;Spiral | — | Passthrough |
| &nbsp;&nbsp;Tangent Intersection | — | Passthrough |
| &nbsp;&nbsp;Point of Intersection | — | Passthrough |
| &nbsp;&nbsp;Cant Information | — | Passthrough |
| Table Styles | — | Passthrough |
| &nbsp;&nbsp;Line | — | Passthrough |
| &nbsp;&nbsp;Curve | — | Passthrough |
| &nbsp;&nbsp;Spiral | — | Passthrough |
| &nbsp;&nbsp;Segment | — | Passthrough |
| Commands | — | Passthrough |

### Profile

| Civil 3D item | code | status |
|---|---|---|
| Profile Styles | — | Passthrough |
| Design Checks | — | Passthrough |
| &nbsp;&nbsp;Design Check Sets | — | Passthrough |
| &nbsp;&nbsp;Line | — | Passthrough |
| &nbsp;&nbsp;Curve | — | Passthrough |
| Label Styles | — | Passthrough |
| &nbsp;&nbsp;Label Sets | — | Passthrough |
| &nbsp;&nbsp;Station | — | Passthrough |
| &nbsp;&nbsp;Grade Breaks | — | Passthrough |
| &nbsp;&nbsp;Line | — | Passthrough |
| &nbsp;&nbsp;Curve | — | Passthrough |
| Commands | — | Passthrough |

### Profile View

| Civil 3D item | code | status |
|---|---|---|
| Profile View Styles | — | Passthrough |
| Label Styles | — | Passthrough |
| &nbsp;&nbsp;Station Elevation | — | Passthrough |
| &nbsp;&nbsp;Depth | — | Passthrough |
| &nbsp;&nbsp;Projection | — | Passthrough |
| &nbsp;&nbsp;Crossing | — | Passthrough |
| Band Styles | — | Passthrough |
| &nbsp;&nbsp;Band Sets | — | Passthrough |
| &nbsp;&nbsp;Profile Data | — | Passthrough |
| &nbsp;&nbsp;Vertical Geometry | — | Passthrough |
| &nbsp;&nbsp;Horizontal Geometry | — | Passthrough |
| &nbsp;&nbsp;Superelevation Data | — | Passthrough |
| &nbsp;&nbsp;Sectional Data | — | Passthrough |
| &nbsp;&nbsp;Cant Data | — | Passthrough |
| &nbsp;&nbsp;Speed | — | Passthrough |
| &nbsp;&nbsp;Point Proximity | — | Passthrough |
| &nbsp;&nbsp;Alignment Proximity | — | Passthrough |
| &nbsp;&nbsp;Pipe Network | — | Passthrough |
| &nbsp;&nbsp;Pressure Network | — | Passthrough |
| Commands | — | Passthrough |

### Superelevation View

| Civil 3D item | code | status |
|---|---|---|
| Superelevation View Styles | — | Passthrough |
| Commands | — | Passthrough |

### Cant View

| Civil 3D item | code | status |
|---|---|---|
| Cant View Styles | — | Passthrough |
| Commands | — | Passthrough |

### Sample Line

| Civil 3D item | code | status |
|---|---|---|
| Sample Line Styles | — | Passthrough |
| Label Styles | — | Passthrough |
| Commands | — | Passthrough |

### Section

| Civil 3D item | code | status |
|---|---|---|
| Section Styles | — | Passthrough |
| Label Styles | — | Passthrough |
| &nbsp;&nbsp;Label Sets | — | Passthrough |
| &nbsp;&nbsp;Major Offset | — | Passthrough |
| &nbsp;&nbsp;Minor Offset | — | Passthrough |
| &nbsp;&nbsp;Grade Break | — | Passthrough |
| &nbsp;&nbsp;Segment | — | Passthrough |
| &nbsp;&nbsp;Corridor Points | — | Passthrough |

### Section View

| Civil 3D item | code | status |
|---|---|---|
| Section View Styles | — | Passthrough |
| Group Plot Styles | — | Passthrough |
| Sheet Styles | — | Passthrough |
| Label Styles | — | Passthrough |
| &nbsp;&nbsp;Offset Elevation | — | Passthrough |
| &nbsp;&nbsp;Grade | — | Passthrough |
| &nbsp;&nbsp;Projection | — | Passthrough |
| &nbsp;&nbsp;Crossing | — | Passthrough |
| Band Styles | — | Passthrough |
| &nbsp;&nbsp;Band Sets | — | Passthrough |
| &nbsp;&nbsp;Section Data | — | Passthrough |
| &nbsp;&nbsp;Section Segment | — | Passthrough |
| Table Styles | — | Passthrough |
| &nbsp;&nbsp;Total Volume | — | Passthrough |
| &nbsp;&nbsp;Material | — | Passthrough |
| Commands | — | Passthrough |

### Mass Haul Line

| Civil 3D item | code | status |
|---|---|---|
| Mass Haul Line Styles | — | Passthrough |

### Mass Haul View

| Civil 3D item | code | status |
|---|---|---|
| Mass Haul View Styles | — | Passthrough |
| Commands | — | Passthrough |

### Rail

| Civil 3D item | code | status |
|---|---|---|
| Commands | — | Passthrough |

### Rail Turnout

| Civil 3D item | code | status |
|---|---|---|
| Turnout Styles | — | Passthrough |
| Label Styles | — | Passthrough |
| &nbsp;&nbsp;Label Sets | — | Passthrough |
| &nbsp;&nbsp;Turnout | — | Passthrough |
| &nbsp;&nbsp;Turnout Profile | — | Passthrough |
| &nbsp;&nbsp;Critical Points | — | Passthrough |
| &nbsp;&nbsp;Critical Points Profile | — | Passthrough |
| &nbsp;&nbsp;Turnout Dimension Profile | — | Passthrough |
| Commands | — | Passthrough |

### Catchment

| Civil 3D item | code | status |
|---|---|---|
| Catchment Styles | — | Passthrough |
| Label Styles | — | Passthrough |
| &nbsp;&nbsp;Area | — | Passthrough |
| &nbsp;&nbsp;Flow Segment | — | Passthrough |
| Commands | — | Passthrough |

### Pipe Network

| Civil 3D item | code | status |
|---|---|---|
| Parts Lists | — | Passthrough |
| Interference Styles | — | Passthrough |
| Commands | — | Passthrough |

### Pipe

| Civil 3D item | code | status |
|---|---|---|
| Pipe Styles | — | Passthrough |
| Pipe Rule Set | — | Passthrough |
| Label Styles | — | Passthrough |
| &nbsp;&nbsp;Plan Profile | — | Passthrough |
| &nbsp;&nbsp;Crossing Section | — | Passthrough |
| &nbsp;&nbsp;Crossing Profile | — | Passthrough |
| Table Styles | — | Passthrough |

### Structure

| Civil 3D item | code | status |
|---|---|---|
| Structure Styles | — | Passthrough |
| Structure Rule Set | — | Passthrough |
| Label Styles | — | Passthrough |
| Table Styles | — | Passthrough |

### Pressure Network

| Civil 3D item | code | status |
|---|---|---|
| Parts Lists | — | Passthrough |
| Commands | — | Passthrough |

### Pressure Pipe

| Civil 3D item | code | status |
|---|---|---|
| Pressure Pipe Styles | — | Passthrough |
| Label Styles | — | Passthrough |
| &nbsp;&nbsp;Plan Profile | — | Passthrough |
| &nbsp;&nbsp;Crossing Section | — | Passthrough |
| &nbsp;&nbsp;Crossing Profile | — | Passthrough |
| Table Styles | — | Passthrough |

### Fitting

| Civil 3D item | code | status |
|---|---|---|
| Fitting Styles | — | Passthrough |
| Label Styles | — | Passthrough |
| Table Styles | — | Passthrough |

### Appurtenance

| Civil 3D item | code | status |
|---|---|---|
| Appurtenance Styles | — | Passthrough |
| Label Styles | — | Passthrough |
| Table Styles | — | Passthrough |

### Bridge

| Civil 3D item | code | status |
|---|---|---|
| Bridge Projection Styles | — | Passthrough |
| Label Styles | — | Passthrough |
| &nbsp;&nbsp;Label Sets | — | Passthrough |
| &nbsp;&nbsp;Deck | — | Passthrough |
| &nbsp;&nbsp;Girder Group | — | Passthrough |
| &nbsp;&nbsp;Girder | — | Passthrough |
| &nbsp;&nbsp;Pier | — | Passthrough |
| &nbsp;&nbsp;Foundation | — | Passthrough |
| &nbsp;&nbsp;Abutment | — | Passthrough |
| &nbsp;&nbsp;Generic Object | — | Passthrough |

### Corridor

| Civil 3D item | code | status |
|---|---|---|
| Corridor Styles | — | Passthrough |
| Commands | — | Passthrough |

### Intersection

| Civil 3D item | code | status |
|---|---|---|
| Intersection Styles | — | Passthrough |
| Label Styles | — | Passthrough |
| &nbsp;&nbsp;Intersection Location Labels | — | Passthrough |
| Commands | — | Passthrough |

### Assembly

| Civil 3D item | code | status |
|---|---|---|
| Assembly Styles | — | Passthrough |
| Commands | — | Passthrough |

### Subassembly

| Civil 3D item | code | status |
|---|---|---|
| Commands | — | Passthrough |

### Quantity Takeoff

| Civil 3D item | code | status |
|---|---|---|
| Quantity Takeoff Criteria | — | Passthrough |
| Table Styles | — | Passthrough |
| &nbsp;&nbsp;Total Volume | — | Passthrough |
| &nbsp;&nbsp;Material | — | Passthrough |
| Commands | — | Passthrough |

### Survey

| Civil 3D item | code | status |
|---|---|---|
| Networks | — | Passthrough |
| &nbsp;&nbsp;Network Styles | — | Passthrough |
| Figures | — | Passthrough |
| &nbsp;&nbsp;Figure Styles | — | Passthrough |
| &nbsp;&nbsp;Label Styles | — | Passthrough |
| &nbsp;&nbsp;&nbsp;&nbsp;Figure | — | Passthrough |
| &nbsp;&nbsp;&nbsp;&nbsp;Line | — | Passthrough |
| &nbsp;&nbsp;&nbsp;&nbsp;Curve | — | Passthrough |
| Commands | — | Passthrough |

### View Frame Group

| Civil 3D item | code | status |
|---|---|---|
| Commands | — | Passthrough |

### View Frame

| Civil 3D item | code | status |
|---|---|---|
| View Frame Styles | — | Passthrough |
| Label Styles | — | Passthrough |
| &nbsp;&nbsp;View Frame | — | Passthrough |

### Match Line

| Civil 3D item | code | status |
|---|---|---|
| Match Line Styles | — | Passthrough |
| Label Styles | — | Passthrough |
| &nbsp;&nbsp;Match Line Left | — | Passthrough |
| &nbsp;&nbsp;Match Line Right | — | Passthrough |

### Building Site

| Civil 3D item | code | status |
|---|---|---|
| Building Site Styles | — | Passthrough |
| Commands | — | Passthrough |
