<script lang="ts">
import { cloneVNode, defineComponent, h } from "vue";

export default defineComponent({
  name: "Primitive",
  inheritAttrs: false,
  props: {
    as: {
      type: String,
      default: "div",
    },
    asChild: {
      type: Boolean,
      default: false,
    },
  },
  setup(props, { attrs, slots }) {
    return () => {
      if (props.asChild) {
        const children = slots.default?.();
        if (!children || children.length === 0) return null;
        return cloneVNode(children[0], attrs);
      }
      return h(props.as, attrs, slots.default?.());
    };
  },
});
</script>