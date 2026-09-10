// go-yaml v3 adapter.
//
// The one peer besides ruamel that models comments: every node carries
// HeadComment, LineComment and FootComment, so it can answer where a
// comment attaches. It is also what yq edits through, which makes it the
// implementation yqr is most often compared against in practice.
package main

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"strings"

	"gopkg.in/yaml.v3"
)

type request struct {
	Op     string        `json:"op"`
	Source string        `json:"source"`
	Path   []interface{} `json:"path"`
}

// walk resolves a path of segments to the node it names.
func walk(n *yaml.Node, path []interface{}) (*yaml.Node, *yaml.Node, int, error) {
	cur := n
	if cur.Kind == yaml.DocumentNode {
		cur = cur.Content[0]
	}
	var parent *yaml.Node
	idx := -1
	for _, seg := range path {
		switch cur.Kind {
		case yaml.MappingNode:
			key := fmt.Sprintf("%v", seg)
			found := false
			for i := 0; i < len(cur.Content); i += 2 {
				if cur.Content[i].Value == key {
					parent, idx = cur, i
					cur = cur.Content[i+1]
					found = true
					break
				}
			}
			if !found {
				return nil, nil, -1, fmt.Errorf("path segment %v not found", seg)
			}
		case yaml.SequenceNode:
			i, ok := seg.(float64)
			if !ok || int(i) >= len(cur.Content) {
				return nil, nil, -1, fmt.Errorf("index %v out of range", seg)
			}
			parent, idx = cur, int(i)
			cur = cur.Content[int(i)]
		default:
			return nil, nil, -1, fmt.Errorf("cannot walk into a %v", cur.Kind)
		}
	}
	return cur, parent, idx, nil
}

func canon(n *yaml.Node) (interface{}, error) {
	var v interface{}
	if err := n.Decode(&v); err != nil {
		return nil, err
	}
	return v, nil
}

func main() {
	raw, _ := io.ReadAll(os.Stdin)
	var req request
	_ = json.Unmarshal(raw, &req)
	out := map[string]interface{}{}
	emit := func() {
		b, _ := json.Marshal(out)
		fmt.Println(string(b))
	}

	if req.Op == "version" {
		out["version"] = "go-yaml v3.0.1"
		emit()
		return
	}

	var doc yaml.Node
	if err := yaml.Unmarshal([]byte(req.Source), &doc); err != nil {
		out["error"] = fmt.Sprintf("%T: %s", err, strings.SplitN(err.Error(), "\n", 2)[0])
		emit()
		return
	}

	switch req.Op {
	case "load":
		v, err := canon(&doc)
		if err != nil {
			out["error"] = err.Error()
			break
		}
		b, _ := json.Marshal(v)
		out["result"] = string(b)
	case "roundtrip":
		b, err := yaml.Marshal(&doc)
		if err != nil {
			out["error"] = err.Error()
			break
		}
		out["result"] = string(b)
	case "comments":
		node, parent, idx, err := walk(&doc, req.Path)
		if err != nil {
			out["error"] = err.Error()
			break
		}
		c := map[string][]string{}
		add := func(label, text string) {
			if text != "" {
				c[label] = append(c[label], strings.TrimSpace(strings.TrimPrefix(text, "#")))
			}
		}
		add("value_line", node.LineComment)
		add("value_head", node.HeadComment)
		add("value_foot", node.FootComment)
		if parent != nil && parent.Kind == yaml.MappingNode {
			k := parent.Content[idx]
			add("key_line", k.LineComment)
			add("key_head", k.HeadComment)
			add("key_foot", k.FootComment)
		}
		b, _ := json.Marshal(c)
		out["result"] = string(b)
	case "set_comment":
		// Where does a set comment land when the entry has no value?
		// go-yaml has a slot on every node, so the question is which one
		// the library treats as the entry's line.
		node, parent, idx, err := walk(&doc, req.Path)
		if err != nil {
			out["error"] = err.Error()
			break
		}
		if parent != nil && parent.Kind == yaml.MappingNode {
			parent.Content[idx].LineComment = "# set"
		} else {
			node.LineComment = "# set"
		}
		b, err := yaml.Marshal(&doc)
		if err != nil {
			out["error"] = err.Error()
			break
		}
		out["result"] = string(b)
	case "delete":
		_, parent, idx, err := walk(&doc, req.Path)
		if err != nil {
			out["error"] = err.Error()
			break
		}
		if parent == nil {
			out["error"] = "cannot delete the document root"
			break
		}
		if parent.Kind == yaml.MappingNode {
			parent.Content = append(parent.Content[:idx], parent.Content[idx+2:]...)
		} else {
			parent.Content = append(parent.Content[:idx], parent.Content[idx+1:]...)
		}
		b, err := yaml.Marshal(&doc)
		if err != nil {
			out["error"] = err.Error()
			break
		}
		out["result"] = string(b)
	default:
		out["error"] = "unknown op " + req.Op
	}
	emit()
}
