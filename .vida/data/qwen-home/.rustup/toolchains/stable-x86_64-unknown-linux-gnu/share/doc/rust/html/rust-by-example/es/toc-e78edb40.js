// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="index.html">Introducción</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="hello.html"><strong aria-hidden="true">1.</strong> Hola Mundo</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="hello/comment.html"><strong aria-hidden="true">1.1.</strong> Comentarios</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="hello/print.html"><strong aria-hidden="true">1.2.</strong> Imprimir con formatos</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="hello/print/print_debug.html"><strong aria-hidden="true">1.2.1.</strong> Depuración</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="hello/print/print_display.html"><strong aria-hidden="true">1.2.2.</strong> Display</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="hello/print/print_display/testcase_list.html"><strong aria-hidden="true">1.2.2.1.</strong> Ejemplot: Lista</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="hello/print/fmt.html"><strong aria-hidden="true">1.2.3.</strong> Formateo</a></span></li></ol></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="primitives.html"><strong aria-hidden="true">2.</strong> Primitivas</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="primitives/literals.html"><strong aria-hidden="true">2.1.</strong> Operadores y Literales</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="primitives/tuples.html"><strong aria-hidden="true">2.2.</strong> Tuplas</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="primitives/array.html"><strong aria-hidden="true">2.3.</strong> Arreglos y slices</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="custom_types.html"><strong aria-hidden="true">3.</strong> Tipos Personalizados</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="custom_types/structs.html"><strong aria-hidden="true">3.1.</strong> Estructuras</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="custom_types/enum.html"><strong aria-hidden="true">3.2.</strong> Enums</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="custom_types/enum/enum_use.html"><strong aria-hidden="true">3.2.1.</strong> use</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="custom_types/enum/c_like.html"><strong aria-hidden="true">3.2.2.</strong> Herencias de C</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="custom_types/enum/testcase_linked_list.html"><strong aria-hidden="true">3.2.3.</strong> Ejemplo: lista ligada</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="custom_types/constants.html"><strong aria-hidden="true">3.3.</strong> constantes</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="variable_bindings.html"><strong aria-hidden="true">4.</strong> Enlaces de variables</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="variable_bindings/mut.html"><strong aria-hidden="true">4.1.</strong> Mutabilidad</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="variable_bindings/scope.html"><strong aria-hidden="true">4.2.</strong> Alcance y sombreo</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="variable_bindings/declare.html"><strong aria-hidden="true">4.3.</strong> Declara primero</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="variable_bindings/freeze.html"><strong aria-hidden="true">4.4.</strong> Congelar</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="types.html"><strong aria-hidden="true">5.</strong> Tipos</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="types/cast.html"><strong aria-hidden="true">5.1.</strong> Casteo</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="types/literals.html"><strong aria-hidden="true">5.2.</strong> Literales</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="types/inference.html"><strong aria-hidden="true">5.3.</strong> Inferencia</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="types/alias.html"><strong aria-hidden="true">5.4.</strong> Aliasing</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="conversion.html"><strong aria-hidden="true">6.</strong> Conversión</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="conversion/from_into.html"><strong aria-hidden="true">6.1.</strong> From e Into</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="conversion/try_from_try_into.html"><strong aria-hidden="true">6.2.</strong> TryFrom y TryInto</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="conversion/string.html"><strong aria-hidden="true">6.3.</strong> De y hacia Strings</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="expression.html"><strong aria-hidden="true">7.</strong> Expresiones</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="flow_control.html"><strong aria-hidden="true">8.</strong> Control de Flujo</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="flow_control/if_else.html"><strong aria-hidden="true">8.1.</strong> if/else</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="flow_control/loop.html"><strong aria-hidden="true">8.2.</strong> loop</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="flow_control/loop/nested.html"><strong aria-hidden="true">8.2.1.</strong> Nesting and labels</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="flow_control/loop/return.html"><strong aria-hidden="true">8.2.2.</strong> Retornar desde bucles</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="flow_control/while.html"><strong aria-hidden="true">8.3.</strong> while</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="flow_control/for.html"><strong aria-hidden="true">8.4.</strong> for y range</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="flow_control/match.html"><strong aria-hidden="true">8.5.</strong> match</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="flow_control/match/destructuring.html"><strong aria-hidden="true">8.5.1.</strong> Destructura</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="flow_control/match/destructuring/destructure_tuple.html"><strong aria-hidden="true">8.5.1.1.</strong> tuplas</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="flow_control/match/destructuring/destructure_slice.html"><strong aria-hidden="true">8.5.1.2.</strong> arreglos/slices</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="flow_control/match/destructuring/destructure_enum.html"><strong aria-hidden="true">8.5.1.3.</strong> enums</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="flow_control/match/destructuring/destructure_pointers.html"><strong aria-hidden="true">8.5.1.4.</strong> punteros/ref</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="flow_control/match/destructuring/destructure_structures.html"><strong aria-hidden="true">8.5.1.5.</strong> structs</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="flow_control/match/guard.html"><strong aria-hidden="true">8.5.2.</strong> Guardias</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="flow_control/match/binding.html"><strong aria-hidden="true">8.5.3.</strong> Enlaces</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="flow_control/if_let.html"><strong aria-hidden="true">8.6.</strong> if let</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="flow_control/let_else.html"><strong aria-hidden="true">8.7.</strong> let-else</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="flow_control/while_let.html"><strong aria-hidden="true">8.8.</strong> while let</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="fn.html"><strong aria-hidden="true">9.</strong> Funciones</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="fn/methods.html"><strong aria-hidden="true">9.1.</strong> Métodos</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="fn/closures.html"><strong aria-hidden="true">9.2.</strong> Closures</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="fn/closures/capture.html"><strong aria-hidden="true">9.2.1.</strong> Capturas</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="fn/closures/input_parameters.html"><strong aria-hidden="true">9.2.2.</strong> Como parámetros de entrada</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="fn/closures/anonymity.html"><strong aria-hidden="true">9.2.3.</strong> Anonimidad de tipos</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="fn/closures/input_functions.html"><strong aria-hidden="true">9.2.4.</strong> Funciones de entrada</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="fn/closures/output_parameters.html"><strong aria-hidden="true">9.2.5.</strong> Como parámetros de salida</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="fn/closures/closure_examples.html"><strong aria-hidden="true">9.2.6.</strong> Ejemplos de std</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="fn/closures/closure_examples/iter_any.html"><strong aria-hidden="true">9.2.6.1.</strong> Iterator::any</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="fn/closures/closure_examples/iter_find.html"><strong aria-hidden="true">9.2.6.2.</strong> Buscando con iteradores</a></span></li></ol></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="fn/hof.html"><strong aria-hidden="true">9.3.</strong> Funciones de Mayor Orden</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="fn/diverging.html"><strong aria-hidden="true">9.4.</strong> Funciones Divergentes</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="mod.html"><strong aria-hidden="true">10.</strong> Módulos</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="mod/visibility.html"><strong aria-hidden="true">10.1.</strong> Visibilidad</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="mod/struct_visibility.html"><strong aria-hidden="true">10.2.</strong> Visibilidad de estructuras</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="mod/use.html"><strong aria-hidden="true">10.3.</strong> La declaración use</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="mod/super.html"><strong aria-hidden="true">10.4.</strong> super y self</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="mod/split.html"><strong aria-hidden="true">10.5.</strong> Jerarquía de archivos</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="crates.html"><strong aria-hidden="true">11.</strong> Crates</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="crates/lib.html"><strong aria-hidden="true">11.1.</strong> Creando una Library</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="crates/using_lib.html"><strong aria-hidden="true">11.2.</strong> Usando un Library</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="cargo.html"><strong aria-hidden="true">12.</strong> Cargo</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="cargo/deps.html"><strong aria-hidden="true">12.1.</strong> Dependencias</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="cargo/conventions.html"><strong aria-hidden="true">12.2.</strong> Convenciones</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="cargo/test.html"><strong aria-hidden="true">12.3.</strong> Tests</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="cargo/build_scripts.html"><strong aria-hidden="true">12.4.</strong> Scripts de Build</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="attribute.html"><strong aria-hidden="true">13.</strong> Atributos</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="attribute/unused.html"><strong aria-hidden="true">13.1.</strong> dead_code</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="attribute/crate.html"><strong aria-hidden="true">13.2.</strong> Crates</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="attribute/cfg.html"><strong aria-hidden="true">13.3.</strong> cfg</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="attribute/cfg/custom.html"><strong aria-hidden="true">13.3.1.</strong> Personalizaciones</a></span></li></ol></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="generics.html"><strong aria-hidden="true">14.</strong> Genéricos</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="generics/gen_fn.html"><strong aria-hidden="true">14.1.</strong> Funciones</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="generics/impl.html"><strong aria-hidden="true">14.2.</strong> Implementación</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="generics/gen_trait.html"><strong aria-hidden="true">14.3.</strong> Traits</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="generics/bounds.html"><strong aria-hidden="true">14.4.</strong> Cotas</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="generics/bounds/testcase_empty.html"><strong aria-hidden="true">14.4.1.</strong> Ejemplo: cotas vacías</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="generics/multi_bounds.html"><strong aria-hidden="true">14.5.</strong> Cotas Múltiples</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="generics/where.html"><strong aria-hidden="true">14.6.</strong> Cláusulas where</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="generics/new_types.html"><strong aria-hidden="true">14.7.</strong> Modismo New Type</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="generics/assoc_items.html"><strong aria-hidden="true">14.8.</strong> Ítems asociados</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="generics/assoc_items/the_problem.html"><strong aria-hidden="true">14.8.1.</strong> El Problema</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="generics/assoc_items/types.html"><strong aria-hidden="true">14.8.2.</strong> Tipos Asociados</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="generics/phantom.html"><strong aria-hidden="true">14.9.</strong> Parámetros Phantom Type</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="generics/phantom/testcase_units.html"><strong aria-hidden="true">14.9.1.</strong> Ejemplo: clarificación de unidades</a></span></li></ol></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="scope.html"><strong aria-hidden="true">15.</strong> Reglas de alcance</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="scope/raii.html"><strong aria-hidden="true">15.1.</strong> RAII</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="scope/move.html"><strong aria-hidden="true">15.2.</strong> Ownership y moves</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="scope/move/mut.html"><strong aria-hidden="true">15.2.1.</strong> Mutabilidad</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="scope/move/partial_move.html"><strong aria-hidden="true">15.2.2.</strong> Moves parciales</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="scope/borrow.html"><strong aria-hidden="true">15.3.</strong> Borrowing</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="scope/borrow/mut.html"><strong aria-hidden="true">15.3.1.</strong> Mutabilidad</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="scope/borrow/alias.html"><strong aria-hidden="true">15.3.2.</strong> Aliasing</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="scope/borrow/ref.html"><strong aria-hidden="true">15.3.3.</strong> El patrón de ref</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="scope/lifetime.html"><strong aria-hidden="true">15.4.</strong> Lifetimes</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="scope/lifetime/explicit.html"><strong aria-hidden="true">15.4.1.</strong> Anotación explícita</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="scope/lifetime/fn.html"><strong aria-hidden="true">15.4.2.</strong> Funciones</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="scope/lifetime/methods.html"><strong aria-hidden="true">15.4.3.</strong> Métodos</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="scope/lifetime/struct.html"><strong aria-hidden="true">15.4.4.</strong> Structs</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="scope/lifetime/trait.html"><strong aria-hidden="true">15.4.5.</strong> Traits</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="scope/lifetime/lifetime_bounds.html"><strong aria-hidden="true">15.4.6.</strong> Cotas</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="scope/lifetime/lifetime_coercion.html"><strong aria-hidden="true">15.4.7.</strong> Coerción</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="scope/lifetime/static_lifetime.html"><strong aria-hidden="true">15.4.8.</strong> Static</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="scope/lifetime/elision.html"><strong aria-hidden="true">15.4.9.</strong> Elisión</a></span></li></ol></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="trait.html"><strong aria-hidden="true">16.</strong> Traits</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="trait/derive.html"><strong aria-hidden="true">16.1.</strong> Derive</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="trait/dyn.html"><strong aria-hidden="true">16.2.</strong> Retornando Traits con dyn</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="trait/ops.html"><strong aria-hidden="true">16.3.</strong> Sobrecarga de Operadores</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="trait/drop.html"><strong aria-hidden="true">16.4.</strong> Drop</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="trait/iter.html"><strong aria-hidden="true">16.5.</strong> Iterators</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="trait/impl_trait.html"><strong aria-hidden="true">16.6.</strong> impl Trait</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="trait/clone.html"><strong aria-hidden="true">16.7.</strong> Clone</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="trait/supertraits.html"><strong aria-hidden="true">16.8.</strong> Supertraits</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="trait/disambiguating.html"><strong aria-hidden="true">16.9.</strong> Desambiguando traits empalmados</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="macros.html"><strong aria-hidden="true">17.</strong> macro_rules!</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="macros/syntax.html"><strong aria-hidden="true">17.1.</strong> Sintaxis</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="macros/designators.html"><strong aria-hidden="true">17.1.1.</strong> Designadores</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="macros/overload.html"><strong aria-hidden="true">17.1.2.</strong> Sobrecarga</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="macros/repeat.html"><strong aria-hidden="true">17.1.3.</strong> Repetición</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="macros/dry.html"><strong aria-hidden="true">17.2.</strong> DRY (Don&#39;t Repeat Yourself)</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="macros/dsl.html"><strong aria-hidden="true">17.3.</strong> DSL (Domain Specific Languages)</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="macros/variadics.html"><strong aria-hidden="true">17.4.</strong> Variádicas</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="error.html"><strong aria-hidden="true">18.</strong> Manejo de Errores</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="error/panic.html"><strong aria-hidden="true">18.1.</strong> panic</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="error/abort_unwind.html"><strong aria-hidden="true">18.2.</strong> abort &amp; unwind</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="error/option_unwrap.html"><strong aria-hidden="true">18.3.</strong> Option &amp; unwrap</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="error/option_unwrap/question_mark.html"><strong aria-hidden="true">18.3.1.</strong> Desempacando Options con ?</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="error/option_unwrap/map.html"><strong aria-hidden="true">18.3.2.</strong> Combinadores: map</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="error/option_unwrap/and_then.html"><strong aria-hidden="true">18.3.3.</strong> Combinadores: and_then</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="error/option_unwrap/defaults.html"><strong aria-hidden="true">18.3.4.</strong> Defaults: or, or_else, get_or_insert, get_or_insert_with</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="error/result.html"><strong aria-hidden="true">18.4.</strong> Result</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="error/result/result_map.html"><strong aria-hidden="true">18.4.1.</strong> map for Result</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="error/result/result_alias.html"><strong aria-hidden="true">18.4.2.</strong> aliases para Result</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="error/result/early_returns.html"><strong aria-hidden="true">18.4.3.</strong> Retornos tempranos</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="error/result/enter_question_mark.html"><strong aria-hidden="true">18.4.4.</strong> Introduciendo ?</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="error/multiple_error_types.html"><strong aria-hidden="true">18.5.</strong> Mùltiples tipos de errores</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="error/multiple_error_types/option_result.html"><strong aria-hidden="true">18.5.1.</strong> Sacando Results de Options</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="error/multiple_error_types/define_error_type.html"><strong aria-hidden="true">18.5.2.</strong> Definiendo un tipo de error</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="error/multiple_error_types/boxing_errors.html"><strong aria-hidden="true">18.5.3.</strong> Boxing de errores</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="error/multiple_error_types/reenter_question_mark.html"><strong aria-hidden="true">18.5.4.</strong> Otros usos de ?</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="error/multiple_error_types/wrap_error.html"><strong aria-hidden="true">18.5.5.</strong> Envolviendo errores</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="error/iter_result.html"><strong aria-hidden="true">18.6.</strong> Iterando sobre Results</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std.html"><strong aria-hidden="true">19.</strong> Tipos de la biblioteca std</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std/box.html"><strong aria-hidden="true">19.1.</strong> Box, stack y heap</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std/vec.html"><strong aria-hidden="true">19.2.</strong> Vectores</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std/str.html"><strong aria-hidden="true">19.3.</strong> Cadenas</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std/option.html"><strong aria-hidden="true">19.4.</strong> Option</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std/result.html"><strong aria-hidden="true">19.5.</strong> Result</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std/result/question_mark.html"><strong aria-hidden="true">19.5.1.</strong> ?</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std/panic.html"><strong aria-hidden="true">19.6.</strong> panic!</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std/hash.html"><strong aria-hidden="true">19.7.</strong> HashMap</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std/hash/alt_key_types.html"><strong aria-hidden="true">19.7.1.</strong> Tipos de llaves personalizadas/alternas</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std/hash/hashset.html"><strong aria-hidden="true">19.7.2.</strong> HashSet</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std/rc.html"><strong aria-hidden="true">19.8.</strong> Rc</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std/arc.html"><strong aria-hidden="true">19.9.</strong> Arc</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std_misc.html"><strong aria-hidden="true">20.</strong> Std misc</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std_misc/threads.html"><strong aria-hidden="true">20.1.</strong> Hilos</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std_misc/threads/testcase_mapreduce.html"><strong aria-hidden="true">20.1.1.</strong> Ejemplo: map-reduce</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std_misc/channels.html"><strong aria-hidden="true">20.2.</strong> Canales</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std_misc/path.html"><strong aria-hidden="true">20.3.</strong> Path</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std_misc/file.html"><strong aria-hidden="true">20.4.</strong> File I/O</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std_misc/file/open.html"><strong aria-hidden="true">20.4.1.</strong> open</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std_misc/file/create.html"><strong aria-hidden="true">20.4.2.</strong> create</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std_misc/file/read_lines.html"><strong aria-hidden="true">20.4.3.</strong> read_lines</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std_misc/process.html"><strong aria-hidden="true">20.5.</strong> Procesos hijos</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std_misc/process/pipe.html"><strong aria-hidden="true">20.5.1.</strong> Pipes</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std_misc/process/wait.html"><strong aria-hidden="true">20.5.2.</strong> Wait</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std_misc/fs.html"><strong aria-hidden="true">20.6.</strong> Operaciones de Sistema Operativo</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std_misc/arg.html"><strong aria-hidden="true">20.7.</strong> Argumentos del Programa</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std_misc/arg/matching.html"><strong aria-hidden="true">20.7.1.</strong> Parse de argumentos</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="std_misc/ffi.html"><strong aria-hidden="true">20.8.</strong> Foreign Function Interface</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="testing.html"><strong aria-hidden="true">21.</strong> Pruebas</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="testing/unit_testing.html"><strong aria-hidden="true">21.1.</strong> Pruebas Unitarias</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="testing/doc_testing.html"><strong aria-hidden="true">21.2.</strong> Pruebas de Documentación</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="testing/integration_testing.html"><strong aria-hidden="true">21.3.</strong> Pruebas de integración</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="testing/dev_dependencies.html"><strong aria-hidden="true">21.4.</strong> Dependencias de desarrollo</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="unsafe.html"><strong aria-hidden="true">22.</strong> Operaciones Unsafe</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="unsafe/asm.html"><strong aria-hidden="true">22.1.</strong> Ensamblador Inline</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="compatibility.html"><strong aria-hidden="true">23.</strong> Compatibilidad</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="compatibility/raw_identifiers.html"><strong aria-hidden="true">23.1.</strong> Identificadores llanos</a></span></li></ol><li class="chapter-item "><span class="chapter-link-wrapper"><a href="meta.html"><strong aria-hidden="true">24.</strong> Meta</a><a class="chapter-fold-toggle"><div>❱</div></a></span><ol class="section"><li class="chapter-item "><span class="chapter-link-wrapper"><a href="meta/doc.html"><strong aria-hidden="true">24.1.</strong> Documentación</a></span></li><li class="chapter-item "><span class="chapter-link-wrapper"><a href="meta/playground.html"><strong aria-hidden="true">24.2.</strong> Playground</a></span></li></ol></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split('#')[0].split('?')[0];
        if (current_page.endsWith('/')) {
            current_page += 'index.html';
        }
        const links = Array.prototype.slice.call(this.querySelectorAll('a'));
        const l = links.length;
        for (let i = 0; i < l; ++i) {
            const link = links[i];
            const href = link.getAttribute('href');
            if (href && !href.startsWith('#') && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The 'index' page is supposed to alias the first chapter in the book.
            if (link.href === current_page
                || i === 0
                && path_to_root === ''
                && current_page.endsWith('/index.html')) {
                link.classList.add('active');
                let parent = link.parentElement;
                while (parent) {
                    if (parent.tagName === 'LI' && parent.classList.contains('chapter-item')) {
                        parent.classList.add('expanded');
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', e => {
            if (e.target.tagName === 'A') {
                const clientRect = e.target.getBoundingClientRect();
                const sidebarRect = this.getBoundingClientRect();
                sessionStorage.setItem('sidebar-scroll-offset', clientRect.top - sidebarRect.top);
            }
        }, { passive: true });
        const sidebarScrollOffset = sessionStorage.getItem('sidebar-scroll-offset');
        sessionStorage.removeItem('sidebar-scroll-offset');
        if (sidebarScrollOffset !== null) {
            // preserve sidebar scroll position when navigating via links within sidebar
            const activeSection = this.querySelector('.active');
            if (activeSection) {
                const clientRect = activeSection.getBoundingClientRect();
                const sidebarRect = this.getBoundingClientRect();
                const currentOffset = clientRect.top - sidebarRect.top;
                this.scrollTop += currentOffset - parseFloat(sidebarScrollOffset);
            }
        } else {
            // scroll sidebar to current active section when navigating via
            // 'next/previous chapter' buttons
            const activeSection = document.querySelector('#mdbook-sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        const sidebarAnchorToggles = document.querySelectorAll('.chapter-fold-toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(el => {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define('mdbook-sidebar-scrollbox', MDBookSidebarScrollbox);


// ---------------------------------------------------------------------------
// Support for dynamically adding headers to the sidebar.

(function() {
    // This is used to detect which direction the page has scrolled since the
    // last scroll event.
    let lastKnownScrollPosition = 0;
    // This is the threshold in px from the top of the screen where it will
    // consider a header the "current" header when scrolling down.
    const defaultDownThreshold = 150;
    // Same as defaultDownThreshold, except when scrolling up.
    const defaultUpThreshold = 300;
    // The threshold is a virtual horizontal line on the screen where it
    // considers the "current" header to be above the line. The threshold is
    // modified dynamically to handle headers that are near the bottom of the
    // screen, and to slightly offset the behavior when scrolling up vs down.
    let threshold = defaultDownThreshold;
    // This is used to disable updates while scrolling. This is needed when
    // clicking the header in the sidebar, which triggers a scroll event. It
    // is somewhat finicky to detect when the scroll has finished, so this
    // uses a relatively dumb system of disabling scroll updates for a short
    // time after the click.
    let disableScroll = false;
    // Array of header elements on the page.
    let headers;
    // Array of li elements that are initially collapsed headers in the sidebar.
    // I'm not sure why eslint seems to have a false positive here.
    // eslint-disable-next-line prefer-const
    let headerToggles = [];
    // This is a debugging tool for the threshold which you can enable in the console.
    let thresholdDebug = false;

    // Updates the threshold based on the scroll position.
    function updateThreshold() {
        const scrollTop = window.pageYOffset || document.documentElement.scrollTop;
        const windowHeight = window.innerHeight;
        const documentHeight = document.documentElement.scrollHeight;

        // The number of pixels below the viewport, at most documentHeight.
        // This is used to push the threshold down to the bottom of the page
        // as the user scrolls towards the bottom.
        const pixelsBelow = Math.max(0, documentHeight - (scrollTop + windowHeight));
        // The number of pixels above the viewport, at least defaultDownThreshold.
        // Similar to pixelsBelow, this is used to push the threshold back towards
        // the top when reaching the top of the page.
        const pixelsAbove = Math.max(0, defaultDownThreshold - scrollTop);
        // How much the threshold should be offset once it gets close to the
        // bottom of the page.
        const bottomAdd = Math.max(0, windowHeight - pixelsBelow - defaultDownThreshold);
        let adjustedBottomAdd = bottomAdd;

        // Adjusts bottomAdd for a small document. The calculation above
        // assumes the document is at least twice the windowheight in size. If
        // it is less than that, then bottomAdd needs to be shrunk
        // proportional to the difference in size.
        if (documentHeight < windowHeight * 2) {
            const maxPixelsBelow = documentHeight - windowHeight;
            const t = 1 - pixelsBelow / Math.max(1, maxPixelsBelow);
            const clamp = Math.max(0, Math.min(1, t));
            adjustedBottomAdd *= clamp;
        }

        let scrollingDown = true;
        if (scrollTop < lastKnownScrollPosition) {
            scrollingDown = false;
        }

        if (scrollingDown) {
            // When scrolling down, move the threshold up towards the default
            // downwards threshold position. If near the bottom of the page,
            // adjustedBottomAdd will offset the threshold towards the bottom
            // of the page.
            const amountScrolledDown = scrollTop - lastKnownScrollPosition;
            const adjustedDefault = defaultDownThreshold + adjustedBottomAdd;
            threshold = Math.max(adjustedDefault, threshold - amountScrolledDown);
        } else {
            // When scrolling up, move the threshold down towards the default
            // upwards threshold position. If near the bottom of the page,
            // quickly transition the threshold back up where it normally
            // belongs.
            const amountScrolledUp = lastKnownScrollPosition - scrollTop;
            const adjustedDefault = defaultUpThreshold - pixelsAbove
                + Math.max(0, adjustedBottomAdd - defaultDownThreshold);
            threshold = Math.min(adjustedDefault, threshold + amountScrolledUp);
        }

        if (documentHeight <= windowHeight) {
            threshold = 0;
        }

        if (thresholdDebug) {
            const id = 'mdbook-threshold-debug-data';
            let data = document.getElementById(id);
            if (data === null) {
                data = document.createElement('div');
                data.id = id;
                data.style.cssText = `
                    position: fixed;
                    top: 50px;
                    right: 10px;
                    background-color: 0xeeeeee;
                    z-index: 9999;
                    pointer-events: none;
                `;
                document.body.appendChild(data);
            }
            data.innerHTML = `
                <table>
                  <tr><td>documentHeight</td><td>${documentHeight.toFixed(1)}</td></tr>
                  <tr><td>windowHeight</td><td>${windowHeight.toFixed(1)}</td></tr>
                  <tr><td>scrollTop</td><td>${scrollTop.toFixed(1)}</td></tr>
                  <tr><td>pixelsAbove</td><td>${pixelsAbove.toFixed(1)}</td></tr>
                  <tr><td>pixelsBelow</td><td>${pixelsBelow.toFixed(1)}</td></tr>
                  <tr><td>bottomAdd</td><td>${bottomAdd.toFixed(1)}</td></tr>
                  <tr><td>adjustedBottomAdd</td><td>${adjustedBottomAdd.toFixed(1)}</td></tr>
                  <tr><td>scrollingDown</td><td>${scrollingDown}</td></tr>
                  <tr><td>threshold</td><td>${threshold.toFixed(1)}</td></tr>
                </table>
            `;
            drawDebugLine();
        }

        lastKnownScrollPosition = scrollTop;
    }

    function drawDebugLine() {
        if (!document.body) {
            return;
        }
        const id = 'mdbook-threshold-debug-line';
        const existingLine = document.getElementById(id);
        if (existingLine) {
            existingLine.remove();
        }
        const line = document.createElement('div');
        line.id = id;
        line.style.cssText = `
            position: fixed;
            top: ${threshold}px;
            left: 0;
            width: 100vw;
            height: 2px;
            background-color: red;
            z-index: 9999;
            pointer-events: none;
        `;
        document.body.appendChild(line);
    }

    function mdbookEnableThresholdDebug() {
        thresholdDebug = true;
        updateThreshold();
        drawDebugLine();
    }

    window.mdbookEnableThresholdDebug = mdbookEnableThresholdDebug;

    // Updates which headers in the sidebar should be expanded. If the current
    // header is inside a collapsed group, then it, and all its parents should
    // be expanded.
    function updateHeaderExpanded(currentA) {
        // Add expanded to all header-item li ancestors.
        let current = currentA.parentElement;
        while (current) {
            if (current.tagName === 'LI' && current.classList.contains('header-item')) {
                current.classList.add('expanded');
            }
            current = current.parentElement;
        }
    }

    // Updates which header is marked as the "current" header in the sidebar.
    // This is done with a virtual Y threshold, where headers at or below
    // that line will be considered the current one.
    function updateCurrentHeader() {
        if (!headers || !headers.length) {
            return;
        }

        // Reset the classes, which will be rebuilt below.
        const els = document.getElementsByClassName('current-header');
        for (const el of els) {
            el.classList.remove('current-header');
        }
        for (const toggle of headerToggles) {
            toggle.classList.remove('expanded');
        }

        // Find the last header that is above the threshold.
        let lastHeader = null;
        for (const header of headers) {
            const rect = header.getBoundingClientRect();
            if (rect.top <= threshold) {
                lastHeader = header;
            } else {
                break;
            }
        }
        if (lastHeader === null) {
            lastHeader = headers[0];
            const rect = lastHeader.getBoundingClientRect();
            const windowHeight = window.innerHeight;
            if (rect.top >= windowHeight) {
                return;
            }
        }

        // Get the anchor in the summary.
        const href = '#' + lastHeader.id;
        const a = [...document.querySelectorAll('.header-in-summary')]
            .find(element => element.getAttribute('href') === href);
        if (!a) {
            return;
        }

        a.classList.add('current-header');

        updateHeaderExpanded(a);
    }

    // Updates which header is "current" based on the threshold line.
    function reloadCurrentHeader() {
        if (disableScroll) {
            return;
        }
        updateThreshold();
        updateCurrentHeader();
    }


    // When clicking on a header in the sidebar, this adjusts the threshold so
    // that it is located next to the header. This is so that header becomes
    // "current".
    function headerThresholdClick(event) {
        // See disableScroll description why this is done.
        disableScroll = true;
        setTimeout(() => {
            disableScroll = false;
        }, 100);
        // requestAnimationFrame is used to delay the update of the "current"
        // header until after the scroll is done, and the header is in the new
        // position.
        requestAnimationFrame(() => {
            requestAnimationFrame(() => {
                // Closest is needed because if it has child elements like <code>.
                const a = event.target.closest('a');
                const href = a.getAttribute('href');
                const targetId = href.substring(1);
                const targetElement = document.getElementById(targetId);
                if (targetElement) {
                    threshold = targetElement.getBoundingClientRect().bottom;
                    updateCurrentHeader();
                }
            });
        });
    }

    // Takes the nodes from the given head and copies them over to the
    // destination, along with some filtering.
    function filterHeader(source, dest) {
        const clone = source.cloneNode(true);
        clone.querySelectorAll('mark').forEach(mark => {
            mark.replaceWith(...mark.childNodes);
        });
        dest.append(...clone.childNodes);
    }

    // Scans page for headers and adds them to the sidebar.
    document.addEventListener('DOMContentLoaded', function() {
        const activeSection = document.querySelector('#mdbook-sidebar .active');
        if (activeSection === null) {
            return;
        }

        const main = document.getElementsByTagName('main')[0];
        headers = Array.from(main.querySelectorAll('h2, h3, h4, h5, h6'))
            .filter(h => h.id !== '' && h.children.length && h.children[0].tagName === 'A');

        if (headers.length === 0) {
            return;
        }

        // Build a tree of headers in the sidebar.

        const stack = [];

        const firstLevel = parseInt(headers[0].tagName.charAt(1));
        for (let i = 1; i < firstLevel; i++) {
            const ol = document.createElement('ol');
            ol.classList.add('section');
            if (stack.length > 0) {
                stack[stack.length - 1].ol.appendChild(ol);
            }
            stack.push({level: i + 1, ol: ol});
        }

        // The level where it will start folding deeply nested headers.
        const foldLevel = 3;

        for (let i = 0; i < headers.length; i++) {
            const header = headers[i];
            const level = parseInt(header.tagName.charAt(1));

            const currentLevel = stack[stack.length - 1].level;
            if (level > currentLevel) {
                // Begin nesting to this level.
                for (let nextLevel = currentLevel + 1; nextLevel <= level; nextLevel++) {
                    const ol = document.createElement('ol');
                    ol.classList.add('section');
                    const last = stack[stack.length - 1];
                    const lastChild = last.ol.lastChild;
                    // Handle the case where jumping more than one nesting
                    // level, which doesn't have a list item to place this new
                    // list inside of.
                    if (lastChild) {
                        lastChild.appendChild(ol);
                    } else {
                        last.ol.appendChild(ol);
                    }
                    stack.push({level: nextLevel, ol: ol});
                }
            } else if (level < currentLevel) {
                while (stack.length > 1 && stack[stack.length - 1].level > level) {
                    stack.pop();
                }
            }

            const li = document.createElement('li');
            li.classList.add('header-item');
            li.classList.add('expanded');
            if (level < foldLevel) {
                li.classList.add('expanded');
            }
            const span = document.createElement('span');
            span.classList.add('chapter-link-wrapper');
            const a = document.createElement('a');
            span.appendChild(a);
            a.href = '#' + header.id;
            a.classList.add('header-in-summary');
            filterHeader(header.children[0], a);
            a.addEventListener('click', headerThresholdClick);
            const nextHeader = headers[i + 1];
            if (nextHeader !== undefined) {
                const nextLevel = parseInt(nextHeader.tagName.charAt(1));
                if (nextLevel > level && level >= foldLevel) {
                    const toggle = document.createElement('a');
                    toggle.classList.add('chapter-fold-toggle');
                    toggle.classList.add('header-toggle');
                    toggle.addEventListener('click', () => {
                        li.classList.toggle('expanded');
                    });
                    const toggleDiv = document.createElement('div');
                    toggleDiv.textContent = '❱';
                    toggle.appendChild(toggleDiv);
                    span.appendChild(toggle);
                    headerToggles.push(li);
                }
            }
            li.appendChild(span);

            const currentParent = stack[stack.length - 1];
            currentParent.ol.appendChild(li);
        }

        const onThisPage = document.createElement('div');
        onThisPage.classList.add('on-this-page');
        onThisPage.append(stack[0].ol);
        const activeItemSpan = activeSection.parentElement;
        activeItemSpan.after(onThisPage);
    });

    document.addEventListener('DOMContentLoaded', reloadCurrentHeader);
    document.addEventListener('scroll', reloadCurrentHeader, { passive: true });
})();

