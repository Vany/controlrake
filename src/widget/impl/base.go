// package widget: Widget is inner representation of functionality, that connects web and server part.
package impl

import (
	"context"
	"fmt"
	"github.com/rs/zerolog"
	"github.com/vany/controlrake/src/widget/api"
	"github.com/vany/pirog"
	"html/template"
	"io"
	"reflect"
)

var TypeRegistry = make(map[string]reflect.Type)           // typename => widgettype
var TemplateStringRegistry = make(map[string]string)       // typename => templatetext
var TemplateRegistry = make(map[string]*template.Template) // typename => html.template

// RegisterWidgetType - For use in actual widgets, we do not know about all widgets
func RegisterWidgetType(w api.Widget, tmpl string) struct{} {
	t := reflect.TypeOf(w).Elem()
	TypeRegistry[t.Name()] = t
	TemplateStringRegistry[t.Name()] = tmpl
	return struct{}{}
}

// BaseWidget - fragile base for all widgets, partially POD
type BaseWidget struct {
	api.WidgetConfig
	Widget api.Widget     // link to actual widget object
	Log    zerolog.Logger // logger for specified widget

	WebSpitton chan string // messages goes to web interface
}

func (w *BaseWidget) Init(context.Context, api.WidgetConstructor) error { return nil }

// Base - interfaced method to access to base data
func (w *BaseWidget) Base() *BaseWidget { return w }

// TemplateWrap - coniguration method, it may be overriden in "subclasses"
func (w *BaseWidget) TemplateWrap(str string) string {
	return fmt.Sprintf(`<div class="widget" id="{{.Name}}" {{if .Style}}style="{{.Style}}"{{end}}>%s</div>`+"\n", str)
}

// Consume websocket message in separate goroutine
func (w *BaseWidget) Dispatch(ctx context.Context, event string) error {
	return w.Errorf("Dispatch() is Virtual")
}

// SendToWeb - Wrap message with one level of path system and send to web
func (w *BaseWidget) SendToWeb(ctx context.Context, msg string) error {
	pirog.SEND(ctx, w.WebSpitton, w.Name+"|"+msg)
	return nil
}

// RenderTo - render for http request
func (w *BaseWidget) RenderTo(ctx context.Context, arg string, wr io.Writer) error {
	if _, ok := TemplateRegistry[w.Type]; !ok {
		if str, ok := TemplateStringRegistry[w.Type]; !ok {
			return w.Errorf("template not found")
		} else if x, err := template.New(w.Type).Funcs(w.Widget.RenderFuncs()).Parse(w.TemplateWrap(str)); err != nil {
			return w.Errorf("template parse error: %w", err)
		} else {
			TemplateRegistry[w.Type] = x
		}
	}
	return TemplateRegistry[w.Type].Execute(wr, w.Widget)
}

// Errorf - format error in widget context
func (w *BaseWidget) Errorf(f string, args ...any) error {
	return fmt.Errorf("%s(%s)"+f, append([]any{w.Type, w.Name}, args...)...)
}

// RenderFuncs - overridable set of template functions
func (w *BaseWidget) RenderFuncs() map[string]any { return make(map[string]any) }
