
{} (:package |test-tuple)
  :configs $ {} (:init-fn |test-tuple.main/main!) (:reload-fn |test-tuple.main/reload!)
  :files $ {}
    |test-tuple.main $ {}
      :defs $ {}
        |main! $ %{} :CodeEntry (:doc |)
          :code $ quote
            defn main! () (log-title "|Testing tuple")
              assert= (:: :parts |1 |23)
                tag-match (destruct-str |123)
                    :none
                    :: :empty
                  (:some s0 ss) (:: :parts s0 ss)
              assert= (:: :empty)
                tag-match (destruct-str |)
                    :none
                    :: :empty
                  (:some s0 ss) (:: :parts s0 ss)
              assert=
                :: :parts 1 $ [] 2 3
                tag-match
                  destruct-list $ [] 1 2 3
                  (:none) (:: :empty)
                  (:some l0 ls) (:: :parts l0 ls)
              assert= (:: :empty)
                tag-match
                  destruct-list $ []
                  (:none) (:: :empty)
                  (:some l0 ls) (:: :parts l0 ls)
              assert= (:: :parts true 2)
                tag-match
                  destruct-set $ #{} 1 2 3
                  (:none) (:: :empty)
                  (:some l0 ls)
                    :: :parts (number? l0) (count ls)
              assert= (:: :empty)
                tag-match
                  destruct-set $ #{}
                  (:none) (:: :empty)
                  (:some l0 ls)
                    :: :parts (number? l0) (count ls)
              assert= (:: :parts true true 1)
                tag-match
                  destruct-map $ &{} :a 1 :b 2
                  (:none) (:: :empty)
                  (:some k0 v0 ms)
                    :: :parts (tag? k0) (number? v0) (count ms)
              assert= (:: :empty)
                tag-match
                  destruct-map $ &{}
                  (:none) (:: :empty)
                  (:some k0 v0 ms)
                    :: :parts $ count ms
      :ns $ %{} :CodeEntry (:doc |)
        :code $ quote
          ns test-tuple.main $ :require
            util.core :refer $ log-title
